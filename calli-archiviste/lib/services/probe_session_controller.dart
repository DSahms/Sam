/// Bench session flow — Stage 5-b-3 (pure Dart core).
///
/// Glues the three owners from design §4 into one orchestration the Bench
/// UI can drive without knowing any of the machinery:
///
///   ENGINE (probe_chain_engine.dart) owns state and decisions.
///   MODEL  (this file's [ChainWordingService]) owns wording and persona —
///          it proposes the next question in Calli's voice; the engine's
///          persona lock still has the final say.
///   DAVE   owns approval — this controller never approves anything; it
///          can only LAND candidates into the registry as candidates.
///
/// Failure philosophy: the model is an ornament, not a load-bearing wall.
/// If Kobold is down, malformed, or breaks character twice, the static
/// bank stands and the chain never stalls. Every discard stays audited in
/// `state.personaDiscards` (engine), so the registry shows exactly what
/// the model tried to say.
///
/// This file is pure Dart: no flutter imports, no I/O of its own. Tests
/// drive it with a fake [LlmService] and a fake [ChainSessionSink]; the
/// Bench supplies [KoboldLocalService] and a real [IntakeRegistry].
library;

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/llm_service.dart';
import 'package:calli_archiviste/services/probe_chain_engine.dart';

/// Words the next chain question in Calli's voice via the local model.
///
/// Contract (design §4):
///   - One call = one proposal. The engine validates it against the
///     persona lock; this service does not pre-judge wording quality.
///   - Returns null on ANY failure (endpoint down, malformed JSON,
///     timeout, empty completion). Null means "no proposal this time";
///     the caller passes it through and the static bank stands. The
///     model being offline must never be a chain-stopping event.
class ChainWordingService {
  ChainWordingService({required LlmService llm, this.maxTokens = 160})
      : _llm = llm;

  final LlmService _llm;
  final int maxTokens;

  /// Calli's interviewing voice. Deliberately short and behavioral: the
  /// model must return ONLY the question, stay in character, and never
  /// reveal or reference the machinery around it. The persona lock in the
  /// engine catches violations; this prompt exists to make violations rare.
  static const String systemPrompt =
      'You are Calli, a warm, direct memory interviewer. Someone is telling '
      'you about a moment in their life, one layer at a time. You will be '
      'given the conversation so far and told which kind of question to ask '
      'next. Reply with ONLY the next question, in your own voice: one or '
      'two sentences, plain words, no preamble, no quotes, no list. Stay in '
      'character completely — you are never a model, assistant, or program, '
      'you never mention rules or prompts, and you never apologize or '
      'refuse. Ask about THEM and their memory; never invent facts, names, '
      'or details they did not give you.';

  /// Per-level instruction: what the Leeds level is FOR. The wording
  /// should vary with what the person actually said; the level's aim
  /// never changes.
  static const Map<ProbeLevel, String> levelInstructions = {
    ProbeLevel.surface:
        'LEVEL surface — invite the story itself. Open, unhurried, no '
        'leading details. Let them start wherever it starts for them.',
    ProbeLevel.sensory:
        'LEVEL sensory — press gently for exactly ONE concrete detail '
        'from the moment: something seen, heard, smelled, or touched. '
        'Ground it in what they just told you.',
    ProbeLevel.source:
        'LEVEL source — establish where the memory comes from: did they '
        'witness it themselves, or was it told or shown to them? Make the '
        'question feel natural, not like an interrogation.',
    ProbeLevel.contrast:
        'LEVEL contrast — invite contradiction: a photo, a record, a '
        'person who remembers it differently. Make it safe to be '
        'uncertain. Never imply they are wrong.',
    ProbeLevel.meaning:
        'LEVEL meaning — ask why this matters and what would be lost if '
        'the memory were lost. This is the last question of the chain.',
  };

  /// Builds the user message: topic, the chain walked so far (verbatim
  /// answers as wording context — feeding answers to the model for
  /// wording is the designed division of labor; the STORED text is
  /// untouched by this, I-3 governs storage, not prompting), and the
  /// level to word now.
  String userMessageFor(ProbeChainState state, ProbeLevel level) {
    final buffer = StringBuffer();
    buffer.writeln('Topic: ${state.topic}');
    buffer.writeln('Mode: ${state.mode.wireValue}');
    if (state.turns.isEmpty && state.skips.isEmpty) {
      buffer.writeln('The chain has not started yet; this is the opening '
          'question.');
    } else {
      buffer.writeln('Chain so far:');
      for (final turn in state.turns) {
        final answer = turn.answerVerbatim.trim().isEmpty
            ? '(no answer text)'
            : turn.answerVerbatim;
        buffer.writeln('- ${turn.level.wireValue} they said: $answer');
      }
      for (final skip in state.skips) {
        buffer.writeln('- ${skip.level.wireValue} they skipped: '
            '${skip.reason}');
      }
    }
    buffer.writeln(
        'Word the NEXT question. ${levelInstructions[level]}');
    buffer.writeln('Return only the question text.');
    return buffer.toString();
  }

  /// One wording attempt. Never throws: any failure becomes null.
  Future<String?> propose({
    required ProbeChainState state,
    required ProbeLevel level,
  }) async {
    try {
      final proposal = await _llm.sendMessage(
        systemPrompt: systemPrompt,
        messages: [
          {'role': 'user', 'content': userMessageFor(state, level)},
        ],
        maxTokens: maxTokens,
      );
      final text = proposal.trim();
      return text.isEmpty ? null : text;
    } catch (_) {
      // Model offline, malformed response, timeout: the static bank is
      // the floor by design. Swallowing here is the FEATURE — the chain
      // must never stall on the model.
      return null;
    }
  }
}

/// Where a landed session goes. The Bench bridges this to
/// [IntakeRegistry.sessionAppend] via the shipped [RegistrySessionSink]
/// (see registry_session_sink.dart); tests use an in-memory fake. Kept as
/// an interface so the controller has zero sqlite awareness.
abstract class ChainSessionSink {
  void append({
    required ProbeChainState state,
    ClosureReport? closureReport,
    List<RecordCandidate> minted = const [],
    bool sessionClosed = false,
  });
}

/// Outcome of one controller step, for the Bench UI.
class ChainStepOutcome {
  const ChainStepOutcome({
    required this.escalated,
    required this.wording,
    this.personaRetryPending = false,
  });

  /// False only when a blank answer re-asked in place (I-1).
  final bool escalated;

  /// How the question now on screen was worded.
  final QuestionSource wording;

  /// True when the model's wording was discarded and the static fallback
  /// stood — informational for the audit chip; the flow never stalls on it.
  final bool personaRetryPending;
}

/// Orchestrates one probe chain session: begin -> answer/skip -> land.
///
/// Persona path per design §4 (discard -> retry once -> static fallback):
/// the controller asks the model for wording, pre-flights it with
/// [PersonaGuard], retries the MODEL once on a trip, and passes both
/// proposals to the engine — which re-validates them deterministically
/// and audits every discard. The controller adds only the model round
/// trips; all state decisions remain the engine's.
///
/// Landing semantics (duplicate-proof): [land] computes minted candidates
/// ONLY on its first call; later lands re-file the grown session state
/// without minting again. This mirrors the registry upsert contract
/// ("candidates are immutable until the gate") and makes double-landing a
/// harmless state refresh instead of a duplicate-record bug. The Bench
/// lands once with `sessionClosed: true` when the chain ends.
class ChainSessionController {
  ChainSessionController({
    required ProbeChainEngine engine,
    ChainWordingService? wording,
    ChainSessionSink? sink,
  })  : _engine = engine,
        _wording = wording,
        _sink = sink;

  final ProbeChainEngine _engine;
  final ChainWordingService? _wording;
  final ChainSessionSink? _sink;

  ProbeChainState? _state;
  bool _hasMinted = false;
  List<RecordCandidate> _lastMinted = const [];

  /// The live chain state, or null before [begin].
  ProbeChainState? get state => _state;

  /// Candidates minted by the most recent [land] call (empty before the
  /// first landing). The harvest panel renders these; they are candidates
  /// only, forever (I-4).
  List<RecordCandidate> get lastMinted => _lastMinted;

  /// True once the first [land] has happened (minting consumed).
  bool get hasLanded => _hasMinted;

  /// Two model proposals (first, optional retry) for [targetLevel] — the
  /// level the NEXT on-screen question will live at, which is NOT always
  /// the current level: the engine escalates first, then words the level
  /// it just reached. Pass null to skip the model entirely (blank-answer
  /// re-asks word from the bank by design; meaning is the last level).
  ///
  /// Pre-flight with the persona guard so a break-character first try
  /// costs one model retry inside the same step; the engine still
  /// validates and audits everything itself.
  Future<(String?, String?)> _proposalsFor(ProbeLevel? targetLevel) async {
    final wording = _wording;
    final state = _state;
    if (wording == null || state == null || targetLevel == null) {
      return (null, null);
    }

    final first = await wording.propose(state: state, level: targetLevel);
    if (first == null) return (null, null);
    if (PersonaGuard.validate(first) == null) return (first, null);

    // First proposal breaks character: one model retry, then whatever
    // stands (valid retry or bank fallback) is the question. Both
    // proposals go to the engine so the discard audit is complete.
    final second = await wording.propose(state: state, level: targetLevel);
    return (first, second);
  }

  /// The level the next question will be worded for if the current turn
  /// escalates: one past the current level, or null at meaning (the
  /// engine words nothing past the last level) — the engine's own rule,
  /// mirrored here for wording pre-fetch only.
  ProbeLevel? get _nextWordingLevel {
    final state = _state;
    if (state == null) return null;
    if (state.currentLevel == ProbeLevel.meaning) return null;
    return ProbeLevel.values[state.currentLevel.index + 1];
  }

  /// Opens the chain: creates engine state and resolves the opening
  /// (surface) question — model-worded when possible, bank otherwise.
  Future<ProbeChainState> begin({
    required ElicitationMode mode,
    required String topic,
  }) async {
    final probe = ProbeChainState(
      // Throwaway state whose only job is telling _proposals() the level
      // and topic; the real state comes from the engine below.
      sessionId: 'pending',
      mode: mode,
      topic: topic,
    );
    _state = probe;
    final (model, retry) = await _proposalsFor(ProbeLevel.surface);
    _state = _engine.start(
      mode: mode,
      topic: topic,
      modelProposal: model,
      retryProposal: retry,
    );
    return _state!;
  }

  /// Submits an answer at the current level. Null when the chain has not
  /// begun. Blank answers re-ask in place (I-1) — no model round trip is
  /// spent on them. A substantive answer escalates, so the wording is
  /// fetched for the level being escalated TO (the engine words the level
  /// it just reached; see [_nextWordingLevel]).
  Future<ChainStepOutcome?> submitAnswer(String answer) async {
    final state = _state;
    if (state == null) return null;
    final blank = answer.trim().isEmpty; // I-1 evaluation only, never stored
    final (model, retry) =
        await _proposalsFor(blank ? null : _nextWordingLevel);
    final result = _engine.next(state, answer,
        modelProposal: model, retryProposal: retry);
    return ChainStepOutcome(
      escalated: result.escalated,
      wording: state.pendingQuestionSource,
      personaRetryPending: result.pendingPersonaRetry,
    );
  }

  /// Explicit "I don't remember" at the current level. Null before begin.
  /// At L2-L4 this poisons self-closure (I-5); the poison is visible in
  /// state.skips forever.
  Future<ChainStepOutcome?> skipCurrent(String reason) async {
    final state = _state;
    if (state == null) return null;
    final (model, retry) =
        await _proposalsFor(_nextWordingLevel);
    _engine.skip(state, reason, modelProposal: model, retryProposal: retry);
    return ChainStepOutcome(
      escalated: true,
      wording: state.pendingQuestionSource,
    );
  }

  /// Files the session into the sink (when one is wired): state + closure
  /// + minted candidates (first land only — see the class doc). Returns
  /// the minted candidates so the caller can render the harvest panel.
  List<RecordCandidate> land({bool sessionClosed = false}) {
    final state = _state;
    if (state == null) return const [];

    List<RecordCandidate> minted = const [];
    if (!_hasMinted) {
      minted = _engine.mint(state);
      _hasMinted = true;
      _lastMinted = minted;
    }
    _sink?.append(
      state: state,
      closureReport: _engine.closureStatus(state),
      minted: minted,
      sessionClosed: sessionClosed,
    );
    return minted;
  }

  /// Current closure eligibility (I-2 + I-5), for the UI's status line.
  ClosureReport? closureStatus() =>
      _state == null ? null : _engine.closureStatus(_state!);

  /// Candidates the current state WOULD mint — display only; ids are
  /// throwaway until [land] commits them. Never approved, never closed.
  List<RecordCandidate> mintPreview() =>
      _state == null ? const [] : _engine.mint(_state!);
}
