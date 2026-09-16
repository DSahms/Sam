import 'package:flutter/services.dart';
import 'package:calli_archiviste/config/api_config.dart';
import 'package:calli_archiviste/config/chapter_catalog.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/models/interview_session.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/services/llm_service.dart';
import 'package:calli_archiviste/services/long_form_narrative_compiler.dart';

/// Service for generating narrative prose from interview transcripts
///
/// Extracted from the donor application (STAGE 4). This is the consumer the
/// three narrative prompts have been waiting for since Stage 2:
/// - assets/prompts/narrative_system.txt        (single-pass short chapters)
/// - assets/prompts/narrative_chunk_system.txt  (long-form chunks)
/// - assets/prompts/narrative_assemble_system.txt (final assembly)
///
/// Generated narratives are cached by [SessionStorage.saveNarrative] with the
/// [narrativeSignatureForSessions] source signature, so hosts can detect when
/// interview content has changed and regenerate.
class NarrativeService {
  final LlmService _llm;
  late final LongFormNarrativeCompiler _longFormCompiler;

  NarrativeService({required LlmService llm}) : _llm = llm {
    _longFormCompiler = LongFormNarrativeCompiler(llm: llm);
  }

  /// Minimum user answers before staged long-form compilation is preferred.
  static const int longFormThresholdAnswers = 12;

  /// Convert all sessions for a chapter into first-person narrative prose.
  Future<String> generateChapterNarrative({
    required LifeChapter chapter,
    required List<InterviewSession> sessions,
    required String userName,
  }) async {
    final totalAnswers = sessions.fold<int>(
      0,
      (sum, s) => sum + s.answerCount,
    );
    if (totalAnswers >= longFormThresholdAnswers) {
      try {
        return await _longFormCompiler.compileChapter(
          chapter: chapter,
          sessions: sessions,
          userName: userName,
        );
      } catch (e) {
        // Fall through to legacy single-pass only if staged compile fails.
      }
    }

    return _generateLegacySinglePass(
      chapter: chapter,
      sessions: sessions,
      userName: userName,
    );
  }

  Future<String> _generateLegacySinglePass({
    required LifeChapter chapter,
    required List<InterviewSession> sessions,
    required String userName,
  }) async {
    try {
      // Load narrative system prompt from assets
      String systemPrompt = await rootBundle.loadString(
        'packages/calli_archiviste/assets/prompts/narrative_system.txt',
      );

      // Replace placeholders
      final chapterName = ChapterCatalog.title(chapter);
      systemPrompt = systemPrompt.replaceAll('{{chapter_name}}', chapterName);
      systemPrompt = systemPrompt.replaceAll('{{user_name}}', userName);

      // Build transcripts from all sessions in chronological order
      final sortedSessions = List<InterviewSession>.from(sessions)
        ..sort((a, b) => a.startedAt.compareTo(b.startedAt));

      final transcriptBuffer = StringBuffer();
      for (int i = 0; i < sortedSessions.length; i++) {
        final session = sortedSessions[i];
        transcriptBuffer.writeln('--- Session ${i + 1} ---');
        for (final message in session.messages) {
          if (message.role == MessageRole.interviewer) {
            transcriptBuffer.writeln('Interviewer: ${message.content}');
          } else {
            transcriptBuffer.writeln('$userName: ${message.content}');
          }
        }
        transcriptBuffer.writeln();
      }

      systemPrompt = systemPrompt.replaceAll(
        '{{transcripts}}',
        transcriptBuffer.toString(),
      );

      // Call the configured LLM with the narrative prompt
      final response = await _llm.sendMessage(
        systemPrompt: systemPrompt,
        messages: [
          {
            'role': 'user',
            'content':
                'Please convert these transcripts into narrative prose.',
          }
        ],
        maxTokens: ApiConfig.narrativeMaxTokens,
        model: ApiConfig.narrativeModel,
      );

      return response;
    } catch (e) {
      return 'Unable to generate narrative at this time. '
          'Please check your connection and try again.';
    }
  }

  /// Regenerate a section of narrative with user feedback.
  Future<String> reviseNarrative({
    required String currentNarrative,
    required String userFeedback,
    required String userName,
  }) async {
    try {
      final systemPrompt =
          'You are revising autobiography prose based on the author\'s feedback.\n'
          'Here is the current narrative:\n$currentNarrative\n\n'
          'The author says: $userFeedback\n\n'
          'Revise the narrative to incorporate this feedback. '
          'Keep everything else the same. '
          'Write in first person as the author.';

      final response = await _llm.sendMessage(
        systemPrompt: systemPrompt,
        messages: [
          {
            'role': 'user',
            'content': 'Please revise the narrative based on my feedback.',
          }
        ],
        maxTokens: ApiConfig.narrativeMaxTokens,
        model: ApiConfig.narrativeModel,
      );

      return response;
    } catch (e) {
      // Return original if revision fails rather than losing the narrative
      return currentNarrative;
    }
  }

  /// Returns a deterministic signature of interview content used for narrative
  /// generation.
  ///
  /// Moved from the donor's narrative provider (UI layer) into the service
  /// world: Calli has no provider layer, and [SessionStorage.saveNarrative]
  /// expects this value as its `sourceSignature`. Stable 32-bit FNV-1a over
  /// chronologically sorted session content, so any edit to any answer changes
  /// the signature and marks the cached narrative stale.
  static String narrativeSignatureForSessions(
    List<InterviewSession> sessions,
  ) {
    final sorted = List<InterviewSession>.from(sessions)
      ..sort((a, b) => a.startedAt.compareTo(b.startedAt));
    final buffer = StringBuffer();
    for (final s in sorted) {
      buffer
        ..write(s.id)
        ..write('|')
        ..write(s.startedAt.toIso8601String())
        ..write('|');
      for (final m in s.messages) {
        buffer
          ..write(m.role.name)
          ..write(':')
          ..write(m.content)
          ..write('\n');
      }
      buffer.write('---\n');
    }
    final text = buffer.toString();
    var hash = 0x811C9DC5;
    for (final c in text.codeUnits) {
      hash ^= c;
      hash = (hash * 0x01000193) & 0xFFFFFFFF;
    }
    return hash.toRadixString(16).padLeft(8, '0');
  }
}
