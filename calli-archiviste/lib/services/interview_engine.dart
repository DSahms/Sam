import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:calli_archiviste/config/chapter_catalog.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/config/pkc_config.dart';
import 'package:calli_archiviste/models/family_member.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/models/photo_metadata.dart';
import 'package:calli_archiviste/models/pkc_interview_evidence.dart';
import 'package:calli_archiviste/models/interview_session.dart';
import 'package:calli_archiviste/config/api_config.dart';
import 'package:calli_archiviste/services/llm_service.dart';
import 'package:calli_archiviste/services/context_compressor.dart';
import 'package:calli_archiviste/services/kobold_local_service.dart';
import 'package:calli_archiviste/services/grounding_policy.dart';
import 'package:calli_archiviste/services/pkc_bridge.dart';
import 'package:calli_archiviste/services/session_storage.dart';

/// Interview engine for managing the interview flow
/// Generates contextual questions and manages the interview session
class InterviewEngine {
  final LlmService _llm;
  final ContextCompressor _contextCompressor;
  final SessionStorage _storage;
  final PKCInterviewBridge _pkcBridge;
  final LlmService? _pkcLocalLlm;
  final bool _pkcRetrievalEnabled;
  final bool _pkcModelEnrichmentEnabled;
  final String? _pkcEnrichmentSourceId;
  final String userName;
  late String _interviewerSystemPrompt;
  late String _followUpSystemPrompt;

  /// Live interviews do not route questions through PKC
  /// enrichment. PKC retrieval/bridge code remains available for tests and
  /// later opt-in work via [generatePkcBackedNextQuestion].
  static const bool livePkcQuestionEnrichment = false;

  // Token budget constants
  static const int _maxContextTokens = 1500;
  static const int _tokenBudgetPerTier = 250;

  InterviewEngine({
    required LlmService llm,
    required ContextCompressor contextCompressor,
    required SessionStorage storage,
    PKCInterviewBridge? pkcBridge,
    LlmService? pkcLocalLlm,
    bool? pkcRetrievalEnabled,
    bool? pkcModelEnrichmentEnabled,
    String? pkcEnrichmentSourceId,
    this.userName = '',
  }) : _llm = llm,
       _contextCompressor = contextCompressor,
       _storage = storage,
       _pkcBridge = pkcBridge ?? LocalProcessPKCInterviewBridge.fromConfig(),
       _pkcLocalLlm = pkcLocalLlm,
       _pkcRetrievalEnabled = pkcRetrievalEnabled ?? PKCConfig.enabled,
       _pkcModelEnrichmentEnabled =
           pkcModelEnrichmentEnabled ?? PKCConfig.modelEnrichmentEnabled,
       _pkcEnrichmentSourceId =
           pkcEnrichmentSourceId ?? PKCConfig.enrichmentSourceId;

  /// Initialize the interview engine by loading system prompts
  Future<void> initialize() async {
    try {
      _interviewerSystemPrompt = await rootBundle.loadString(
        'assets/prompts/interviewer_system.txt',
      );
      _followUpSystemPrompt = await rootBundle.loadString(
        'assets/prompts/follow_up_system.txt',
      );
    } catch (e) {
      // DEGRADED-MODE SAFETY NET (2026-09-15): the real donor prompt assets
      // now ship in assets/prompts/ and are declared in pubspec.yaml. This
      // fallback only triggers if asset loading fails at runtime (e.g.
      // missing bundle assets). Keep it minimal so the engine stays
      // operable, but surface the failure loudly.
      debugPrint(
        'InterviewEngine.initialize: prompt asset load FAILED, '
        'using placeholder fallback: $e',
      );
      _interviewerSystemPrompt = 'You are conducting a warm, patient life-story interview about {{chapter_name}} ({{chapter_description}}). Ask exactly one question at a time. Build only on what the person has actually told you; never invent details about their life.\n\n{{context_block}}';
      _followUpSystemPrompt = 'You are continuing the same life-story interview. Ask exactly one natural follow-up question that builds only on what the person has actually said in this conversation. Never invent details about their life.';
    }
  }

  /// Retrieve optional PKC evidence for the next interview turn.
  ///
  /// This production boundary intentionally stops at the clean conversational
  /// payload. It does not invoke [_llm] or add evidence to a model prompt.
  Future<PKCInterviewEvidence> preparePkcEvidenceForNextQuestion(
    PKCInterviewRequest request,
  ) {
    return _pkcBridge.retrieveEvidence(request);
  }

  bool get pkcModelEnrichmentActive =>
      livePkcQuestionEnrichment &&
      _pkcRetrievalEnabled &&
      _pkcModelEnrichmentEnabled &&
      _pkcEnrichmentSourceId != null;

  /// Run the complete local PKC-backed follow-up path.
  ///
  /// Authorization is completed before the sanitized payload is appended to
  /// the normal follow-up prompt. This method never uses [_llm].
  Future<PKCBackedFollowUpResult> generatePkcBackedNextQuestion(
    InterviewSession session,
    List<Message> recentMessages, {
    List<FamilyMember>? familyMembers,
    List<PhotoMetadata>? photos,
  }) async {
    final sourceId = _pkcEnrichmentSourceId;
    if (!_pkcRetrievalEnabled ||
        !_pkcModelEnrichmentEnabled ||
        sourceId == null) {
      return PKCBackedFollowUpResult(
        followUp: 'What was that like?',
        evidence: PKCInterviewEvidence.unavailable('disabled'),
        usedPkcContext: false,
        localModelAttempted: false,
        usedFallback: true,
      );
    }

    final interviewerQuestions = recentMessages
        .where((message) => message.role == MessageRole.interviewer)
        .map((message) => message.content)
        .toList();
    final lastQuestion = interviewerQuestions.isEmpty
        ? null
        : interviewerQuestions.last;
    final chapterTitle = ChapterCatalog.title(session.chapter);
    final chapterSubtitle = ChapterCatalog.subtitle(session.chapter);
    final evidence = await preparePkcEvidenceForNextQuestion(
      PKCInterviewRequest(
        operation: PKCInterviewOperation.protectedRetrieval,
        question:
            lastQuestion ?? 'Continue the $chapterTitle interview naturally.',
        chapterContext: '$chapterTitle — $chapterSubtitle',
        requestedContentLevel: 'full_text',
        consentState: 'standing_authorization',
        sourceId: sourceId,
      ),
    );

    String nameContext = '';
    if (userName.isNotEmpty) {
      nameContext =
          'The person you are interviewing is named '
          '$userName. Use their name occasionally and naturally, '
          'as a friend would. Never say "the person" - always '
          'use their name or "you".\n\n';
    }
    var contextBlock = await _buildContext(
      session,
      familyMembers: familyMembers,
      photos: photos,
    );
    contextBlock = nameContext + contextBlock;

    final usePkcContext =
        evidence.ok &&
        evidence.authorized &&
        evidence.canonicalHashVerified &&
        evidence.protectedContentMaterialized &&
        evidence.conversationalPayload.isNotEmpty;
    debugPrint(
      'PKC ENRICHMENT: sources=${evidence.sourceHandles.join(",")} '
      'authorized=${evidence.authorized} '
      'canonicalHashVerified=${evidence.canonicalHashVerified} '
      'materialized=${evidence.protectedContentMaterialized} '
      'contextAdded=$usePkcContext',
    );
    if (usePkcContext) {
      contextBlock +=
          '\n\n## Authorized source-grounded interview context\n'
          '${evidence.conversationalPayload}\n'
          'Use this naturally to ask one relevant follow-up. '
          'Do not mention sources, retrieval, authorization, or internal systems.';
    }

    final systemPrompt =
        '$_followUpSystemPrompt\n\n${GroundingPolicy.promptBlock}\n\n$contextBlock';
    final apiMessages = recentMessages
        .map(
          (message) => {
            'role': message.role == MessageRole.user ? 'user' : 'assistant',
            'content': message.content,
          },
        )
        .toList();

    try {
      final localLlm = _pkcLocalLlm ?? KoboldLocalService.fromConfig();
      final followUp = await localLlm.sendMessage(
        systemPrompt: systemPrompt,
        messages: apiMessages,
      );
      if (followUp.isEmpty) {
        throw StateError('Local model returned an empty follow-up.');
      }
      return PKCBackedFollowUpResult(
        followUp: followUp,
        evidence: evidence,
        usedPkcContext: usePkcContext,
        localModelAttempted: true,
        usedFallback: false,
      );
    } on Object {
      return PKCBackedFollowUpResult(
        followUp: 'What was that like?',
        evidence: evidence,
        usedPkcContext: usePkcContext,
        localModelAttempted: true,
        usedFallback: true,
      );
    }
  }

  /// Generate the opening question for a new session
  Future<String> generateOpeningQuestion(
    LifeChapter chapter,
    String? contextFromPreviousSessions,
  ) async {
    final chapterTitle = ChapterCatalog.title(chapter);
    final chapterDescription = ChapterCatalog.subtitle(chapter);
    final seedQuestions = ChapterCatalog.seedQuestions(chapter);

    // Donor behavior note: a new chapter starts on the first seed, then
    // follow-ups adapt to the answer. Do not send the opener through PKC.
    if (contextFromPreviousSessions == null ||
        contextFromPreviousSessions.isEmpty) {
      if (seedQuestions.isNotEmpty) {
        return seedQuestions[0];
      }
    }

    // Add name context
    String nameContext = '';
    if (userName.isNotEmpty) {
      nameContext =
          'The person you are interviewing is named '
          '$userName. Use their name occasionally and naturally, '
          'as a friend would. Never say "the person" - always '
          'use their name or "you".\n\n';
    }

    // Build context block for opening (includes family and photos)
    String contextBlock = nameContext;
    if (contextFromPreviousSessions != null &&
        contextFromPreviousSessions.isNotEmpty) {
      contextBlock +=
          '## What we have discussed before in this chapter:\n'
          '$contextFromPreviousSessions\n\n'
          'IMPORTANT: The person has already shared the above. '
          'Do NOT ask about topics already covered. Ask about '
          'something NEW that has not been discussed yet. Build '
          'on what you know but explore fresh ground.\n\n'
          'Stay focused on this chapter: $chapterTitle. '
          'Do not ask questions that belong in other life chapters. '
          'Your opening question must be directly about '
          '$chapterDescription.\n\n---\n\n';
    }

    // Add full context (family, photos, cross-chapter summaries)
    // Create a temporary session object for context building
    final tempSession = InterviewSession(
      id: 'temp-opening',
      chapter: chapter,
      startedAt: DateTime.now(),
      messages: [],
    );
    final fullContext = await _buildContext(tempSession);
    contextBlock += fullContext;

    // Replace templates in system prompt
    final systemPrompt = _interviewerSystemPrompt
        .replaceAll('{{chapter_name}}', chapterTitle)
        .replaceAll('{{chapter_description}}', chapterDescription)
        .replaceAll('{{context_block}}', contextBlock);

    try {
      final openingLlm = pkcModelEnrichmentActive
          ? (_pkcLocalLlm ?? KoboldLocalService.fromConfig())
          : _llm;
      final messages = [
        {
          'role': 'user',
          'content': contextFromPreviousSessions != null
              ? 'I am ready to continue. Ask me something we '
                    'have not talked about yet.'
              : 'I am ready to begin.',
        },
      ];

      final question = await openingLlm.sendMessage(
        systemPrompt: systemPrompt,
        messages: messages,
        model: pkcModelEnrichmentActive ? null : ApiConfig.interviewModel,
      );

      return question;
    } catch (e) {
      // Fallback to seed question if API fails
      if (seedQuestions.isNotEmpty) {
        return seedQuestions[0];
      }
      return 'Tell me about your experience with ${chapterTitle.toLowerCase()}.';
    }
  }

  /// Stream the next question based on conversation context.
  /// Yields text chunks as they arrive from the API.
  /// Falls back to non-streaming on error.
  Stream<String> generateNextQuestionStream(
    InterviewSession session,
    List<Message> recentMessages, {
    List<FamilyMember>? familyMembers,
    List<PhotoMetadata>? photos,
  }) async* {
    if (pkcModelEnrichmentActive) {
      final result = await generatePkcBackedNextQuestion(
        session,
        recentMessages,
        familyMembers: familyMembers,
        photos: photos,
      );
      yield result.followUp;
      return;
    }

    // Reuse the same context-building as generateNextQuestion
    String nameContext = '';
    if (userName.isNotEmpty) {
      nameContext =
          'The person you are interviewing is named '
          '$userName. Use their name occasionally and naturally, '
          'as a friend would. Never say "the person" - always '
          'use their name or "you".\n\n';
    }

    var contextBlock = await _buildContext(
      session,
      familyMembers: familyMembers,
      photos: photos,
    );
    contextBlock = nameContext + contextBlock;
    final systemPrompt = _followUpSystemPrompt + '\n\n' + contextBlock;

    final apiMessages = recentMessages.map((m) {
      return {
        'role': m.role == MessageRole.user ? 'user' : 'assistant',
        'content': m.content,
      };
    }).toList();

    try {
      yield* _llm.streamMessage(
        systemPrompt: systemPrompt,
        messages: apiMessages,
        model: ApiConfig.interviewModel,
      );
    } catch (e) {
      // Fall back to non-streaming
      final result = await generateNextQuestion(
        session,
        recentMessages,
        familyMembers: familyMembers,
        photos: photos,
      );
      yield result;
    }
  }

  /// Generate the next question based on conversation context
  Future<String> generateNextQuestion(
    InterviewSession session,
    List<Message> recentMessages, {
    List<FamilyMember>? familyMembers,
    List<PhotoMetadata>? photos,
  }) async {
    if (pkcModelEnrichmentActive) {
      final result = await generatePkcBackedNextQuestion(
        session,
        recentMessages,
        familyMembers: familyMembers,
        photos: photos,
      );
      return result.followUp;
    }

    final chapterTitle = ChapterCatalog.title(session.chapter);

    // Add name context
    String nameContext = '';
    if (userName.isNotEmpty) {
      nameContext =
          'The person you are interviewing is named '
          '$userName. Use their name occasionally and naturally, '
          'as a friend would. Never say "the person" - always '
          'use their name or "you".\n\n';
    }

    // Build full context for this session
    var contextBlock = await _buildContext(
      session,
      familyMembers: familyMembers,
      photos: photos,
    );
    contextBlock = nameContext + contextBlock;

    // Build follow-up system prompt with context block included
    // Note: follow_up_system.txt does NOT have {{context_block}} placeholder
    // so we append the context directly to ensure it's in the system prompt
    final systemPrompt = _followUpSystemPrompt + '\n\n' + contextBlock;

    // Build message history for API call
    final apiMessages = recentMessages.map((m) {
      return {
        'role': m.role == MessageRole.user ? 'user' : 'assistant',
        'content': m.content,
      };
    }).toList();

    try {
      final question = await _llm.sendMessage(
        systemPrompt: systemPrompt,
        messages: apiMessages,
        model: ApiConfig.interviewModel,
      );

      return question;
    } catch (e) {
      debugPrint('Interview follow-up fallback: $e');
      // Fallback to generic follow-up
      return 'What was that like?';
    }
  }

  /// Build family context for interview (Tier 5)
  String _buildFamilyContext(List<FamilyMember> members, LifeChapter chapter) {
    if (members.isEmpty) {
      return '';
    }

    // Filter by chapter relevance
    List<FamilyMember> relevant;
    final lower = (String s) => s.toLowerCase();
    switch (chapter) {
      case LifeChapter.childhood:
        relevant = members.where((m) {
          final rel = lower(m.relationship);
          return rel.contains('mother') ||
              rel.contains('father') ||
              rel.contains('parent') ||
              rel.contains('sibling') ||
              rel.contains('sister') ||
              rel.contains('brother') ||
              rel.contains('grandm') ||
              rel.contains('grandf') ||
              rel.contains('grandp');
        }).toList();
        break;
      case LifeChapter.adolescence:
        relevant = members.where((m) {
          final rel = lower(m.relationship);
          return rel.contains('mother') ||
              rel.contains('father') ||
              rel.contains('parent') ||
              rel.contains('sibling') ||
              rel.contains('sister') ||
              rel.contains('brother') ||
              rel.contains('friend');
        }).toList();
        break;
      case LifeChapter.love:
        // Everyone is relevant to love — parents shape how you love,
        // siblings are part of the story
        relevant = members;
        break;
      case LifeChapter.parenthood:
        relevant = members.where((m) {
          final rel = lower(m.relationship);
          return rel.contains('child') ||
              rel.contains('son') ||
              rel.contains('daughter') ||
              rel.contains('wife') ||
              rel.contains('husband') ||
              rel.contains('partner') ||
              rel.contains('spouse');
        }).toList();
        break;
      case LifeChapter.loss:
        // For loss chapter, include all members (any could be relevant)
        relevant = members;
        break;
      default:
        // For all other chapters, include all family members
        relevant = members;
    }

    if (relevant.isEmpty) {
      return '';
    }

    final formatted = relevant
        .map((m) => '- ${m.toContextString()}')
        .join('\n');

    final buffer =
        '## Family members you have told us about:\n'
        'Use this information naturally — reference people by name,\n'
        'ask about them as if you already know who they are.\n\n'
        '$formatted\n';

    return buffer;
  }

  /// Build photo context for interview (Tier 6)
  String _buildPhotoContext(List<PhotoMetadata> photos) {
    if (photos.isEmpty) {
      return '';
    }

    final formatted = photos
        .where((p) {
          // Only include photos with caption or people info
          return (p.caption != null && p.caption!.isNotEmpty) ||
              p.peopleInPhoto.isNotEmpty;
        })
        .map((p) {
          final parts = <String>[];
          if (p.caption != null && p.caption!.isNotEmpty) {
            parts.add(p.caption!);
          }
          if (p.peopleInPhoto.isNotEmpty) {
            parts.add('People: ${p.peopleInPhoto.join(", ")}');
          }
          if (p.year != null && p.year!.isNotEmpty) {
            parts.add('Year: ${p.year}');
          }
          return '- ${parts.join('. ')}';
        })
        .join('\n');

    if (formatted.isEmpty) {
      return '';
    }

    final buffer =
        '\n## Photos you have shared for this chapter:\n'
        'Reference these photos naturally in conversation —\n'
        'ask about the people, the moment, or what they remember.\n\n'
        '$formatted\n';

    return buffer;
  }

  /// Build contextual information assembled from previous sessions
  /// Follows the 6-tier context assembly as specified in architecture
  Future<String> _buildContext(
    InterviewSession session, {
    List<FamilyMember>? familyMembers,
    List<PhotoMetadata>? photos,
  }) async {
    final contextParts = <String>[];

    // TIER 1: Current session so far (already in messages,
    // no need to summarize - Claude sees the full transcript)

    // TIER 2: Prior completed sessions in same chapter
    final priorSessions = await _storage.getSessionsForChapter(session.chapter);
    final completedPrior = priorSessions
        .where((s) => s.isComplete && s.summary != null && s.id != session.id)
        .toList();
    if (completedPrior.isNotEmpty) {
      final summaries = completedPrior.map((s) => s.summary!).join('\n\n');
      contextParts.add(
        '## What has been shared before in this chapter:\n$summaries\n',
      );
    }

    // TIER 3: Cross-chapter context (other chapters)
    final allChapters = ChapterCatalog.chapters;
    final crossChapterParts = <String>[];
    for (final ch in allChapters) {
      if (ch == session.chapter) continue;
      final chSessions = await _storage.getSessionsForChapter(ch);
      final completed = chSessions
          .where((s) => s.isComplete && s.summary != null)
          .toList();
      if (completed.isNotEmpty) {
        final chTitle = ChapterCatalog.title(ch);
        final lastSummary = completed.last.summary!;
        crossChapterParts.add('$chTitle: $lastSummary');
      }
    }
    if (crossChapterParts.isNotEmpty) {
      contextParts.add(
        '## What has been shared in other chapters:\n'
        '${crossChapterParts.join('\n\n')}\n\n'
        'Use this context to make natural connections when '
        'relevant, but keep your questions focused on the '
        'current chapter topic. Do not ask questions that '
        'belong in another chapter.\n',
      );
    }

    // TIER 4: (implicit - summaries are Tier 4)

    // TIER 5: Family context
    if (familyMembers != null && familyMembers.isNotEmpty) {
      contextParts.add(_buildFamilyContext(familyMembers, session.chapter));
    } else {
      // Load from storage if not provided
      final members = await _storage.getAllFamilyMembers();
      if (members.isNotEmpty) {
        contextParts.add(_buildFamilyContext(members, session.chapter));
      }
    }

    // TIER 6: Photo context
    if (photos != null && photos.isNotEmpty) {
      contextParts.add(_buildPhotoContext(photos));
    } else {
      // Load from storage if not provided
      final chapterPhotos = await _storage.getPhotosForChapter(session.chapter);
      if (chapterPhotos.isNotEmpty) {
        contextParts.add(_buildPhotoContext(chapterPhotos));
      }
    }

    final contextBlock = contextParts.join('\n');
    return contextBlock;
  }

  /// Determine if interviewer should probe deeper on current thread
  /// Returns true if conversation should go deeper on current topic
  bool _shouldProbeDeeper(List<Message> recentMessages, int answerCount) {
    // Probe deeper if:
    // 1. Person gave substantive answer (not just facts)
    // 2. We haven't asked about this topic 3+ times
    // 3. There are emotional indicators (words like "felt", "remember", "surprised")

    if (recentMessages.isEmpty || answerCount < 2) {
      return false;
    }

    // Get last user message
    final lastUserMsg = recentMessages.whereType<Message>().lastWhere(
      (m) => m.role == MessageRole.user,
      orElse: () => Message(role: MessageRole.user, content: ''),
    );

    if (lastUserMsg.content.isEmpty) {
      return false;
    }

    // Check for emotional depth indicators
    final emotionalIndicators = [
      'felt',
      'remember',
      'surprised',
      'shocked',
      'loved',
      'hated',
      'scared',
      'excited',
      'heartbroken',
      'proud',
    ];

    final hasEmotionalDepth = emotionalIndicators.any(
      (indicator) => lastUserMsg.content.toLowerCase().contains(indicator),
    );

    // Check for length (longer answers suggest engagement)
    final hasSubstanceLength = lastUserMsg.content.split(' ').length > 15;

    return hasEmotionalDepth || hasSubstanceLength;
  }
}
