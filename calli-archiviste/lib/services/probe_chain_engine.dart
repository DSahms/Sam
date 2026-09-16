/// The Leeds probe chain engine — Stage 5-b-1.
///
/// Pure Dart state machine for the five-level elicitation chain
/// (surface -> sensory pin -> source -> contrast -> meaning). No I/O, no
/// model calls, no Flutter imports: it runs offline in tests, and the Bench
/// (Stage 5-b-3) drives it with model-proposed wording when available.
///
/// Division of labor (Stage 5-b design §4):
///   ENGINE owns state and decisions — deterministic, testable, model-free.
///   MODEL  owns wording and persona — proposals come in, get validated,
///          and are discarded if they break character.
///   DAVE   owns approval — the engine cannot approve, ever (I-4).
///
/// The five invariants this file enforces (design §2):
///   I-1  Escalation gate: a level advances only on a substantive answer
///        (non-blank) or an explicit skip. Blank answers re-ask in place.
///   I-2  Closure rule: the code path cannot close below contrast (L4).
///        Only Dave can close, manually, stamped in gate_log (5-b-2).
///   I-3  Verbatim fidelity: answers are stored exactly as received. The
///        engine never trims, normalizes, or rephrases canonical text.
///   I-4  The engine cannot approve: mint() produces candidates, nothing
///        else, forever.
///   I-5  Skip poisons closure: any explicit skip at sensory/source/
///        contrast (L2-L4) makes self-closure impossible even if the chain
///        later reaches meaning (L5).
library;

import 'package:calli_archiviste/models/record_candidate.dart';
import 'package:calli_archiviste/services/question_banks.dart';

/// The five Leeds levels, in escalation order. Index == depth.
enum ProbeLevel {
  surface('surface'),
  sensory('sensory'),
  source('source'),
  contrast('contrast'),
  meaning('meaning');

  const ProbeLevel(this.wireValue);
  final String wireValue;

  static ProbeLevel fromWire(String value) => ProbeLevel.values.firstWhere(
    (l) => l.wireValue == value,
    orElse: () => ProbeLevel.surface,
  );
}

/// What kind of material the session is eliciting around. Drives Bench
/// presentation (5-b-3) and registry session rows (5-b-2); the chain
/// itself is mode-agnostic.
enum ElicitationMode {
  photo('photo'),
  document('document'),
  audio('audio'),
  freeRecall('free_recall'),
  chapter('chapter');

  const ElicitationMode(this.wireValue);
  final String wireValue;
}

/// Where the wording of a question came from.
enum QuestionSource {
  modelProposal('model'),
  retryProposal('model_retry'),
  staticBank('static_bank'),
  reask('reask');

  const QuestionSource(this.wireValue);
  final String wireValue;
}

/// One answered turn. [answerVerbatim] is the law (I-3): stored exactly as
/// received, spaces and all. Question wording provenance is tracked so the
/// registry audit can show model vs bank for every turn.
class ProbeTurn {
  const ProbeTurn({
    required this.level,
    required this.question,
    required this.answerVerbatim,
    required this.at,
    required this.questionSource,
  });

  final ProbeLevel level;

  /// The exact question that was on screen when this answer was given.
  final String question;
  final String answerVerbatim;
  final DateTime at;
  final QuestionSource questionSource;

  Map<String, Object?> toJson() => {
    'level': level.wireValue,
    'question': question,
    'answer_verbatim': answerVerbatim,
    'at': at.toIso8601String(),
    'question_source': questionSource.wireValue,
  };

  static ProbeTurn fromJson(Map<String, dynamic> json) {
    final at = json['at'];
    return ProbeTurn(
      level: ProbeLevel.fromWire(json['level'] as String? ?? 'surface'),
      question: json['question'] as String? ?? '',
      answerVerbatim: json['answer_verbatim'] as String? ?? '',
      at: at is String
          ? (DateTime.tryParse(at)?.toUtc() ??
              DateTime.fromMillisecondsSinceEpoch(0, isUtc: true))
          : DateTime.fromMillisecondsSinceEpoch(0, isUtc: true),
      questionSource: QuestionSource.values.firstWhere(
        (s) => s.wireValue == (json['question_source'] as String? ?? ''),
        orElse: () => QuestionSource.staticBank,
      ),
    );
  }
}

/// An explicit "I don't remember" at a level. Chains may attempt every
/// level, but a skip can never be forced past — and at L2-L4 it poisons
/// self-closure (I-5). Skips are recorded, never hidden.
class SkipEvent {
  const SkipEvent({
    required this.level,
    required this.reason,
    required this.at,
  });

  final ProbeLevel level;
  final String reason;
  final DateTime at;

  Map<String, Object?> toJson() => {
    'level': level.wireValue,
    'reason': reason,
    'at': at.toIso8601String(),
  };

  static SkipEvent fromJson(Map<String, dynamic> json) {
    final at = json['at'];
    return SkipEvent(
      level: ProbeLevel.fromWire(json['level'] as String? ?? 'surface'),
      reason: json['reason'] as String? ?? '',
      at: at is String
          ? (DateTime.tryParse(at)?.toUtc() ??
              DateTime.fromMillisecondsSinceEpoch(0, isUtc: true))
          : DateTime.fromMillisecondsSinceEpoch(0, isUtc: true),
    );
  }
}

/// A model proposal that failed the persona lock. Kept in the state so the
/// registry audit shows the discard (design §4: break-character ->
/// discard -> retry once -> static fallback).
class PersonaDiscard {
  const PersonaDiscard({
    required this.level,
    required this.reason,
    required this.at,
  });

  final ProbeLevel level;
  final String reason;
  final DateTime at;

  Map<String, Object?> toJson() => {
    'level': level.wireValue,
    'reason': reason,
    'at': at.toIso8601String(),
  };

  static PersonaDiscard fromJson(Map<String, dynamic> json) {
    final at = json['at'];
    return PersonaDiscard(
      level: ProbeLevel.fromWire(json['level'] as String? ?? 'surface'),
      reason: json['reason'] as String? ?? '',
      at: at is String
          ? (DateTime.tryParse(at)?.toUtc() ??
              DateTime.fromMillisecondsSinceEpoch(0, isUtc: true))
          : DateTime.fromMillisecondsSinceEpoch(0, isUtc: true),
    );
  }
}

/// Mutable chain state. The engine owns the logic; this object owns the
/// data and is what the registry layer (5-b-2) will persist per session.
class ProbeChainState {
  ProbeChainState({
    required this.sessionId,
    required this.mode,
    required this.topic,
    DateTime? startedAt,
  }) : startedAt = startedAt ?? DateTime.now().toUtc();

  final String sessionId;
  final ElicitationMode mode;
  final String topic;
  final DateTime startedAt;

  ProbeLevel currentLevel = ProbeLevel.surface;

  /// The question currently on screen: what [ProbeChainEngine.next] will
  /// bind the next answer to. Set by start/skip/next; persisted so a
  /// session survives a Bench restart.
  String pendingQuestion = '';
  QuestionSource pendingQuestionSource = QuestionSource.staticBank;

  /// How many times the current level has produced wording (advances on
  /// re-asks too, so re-asks rotate variants instead of repeating).
  int levelTurnIndex = 0;

  final List<ProbeTurn> turns = [];
  final List<SkipEvent> skips = [];
  final List<PersonaDiscard> personaDiscards = [];

  /// Deepest level with a recorded substantive turn.
  ProbeLevel get reachedLevel =>
      turns.isEmpty ? ProbeLevel.surface : turns.last.level;

  bool get isComplete =>
      turns.any((t) => t.level == ProbeLevel.meaning) ||
      skips.any((s) => s.level == ProbeLevel.meaning);

  bool hasSkipAt(ProbeLevel level) => skips.any((s) => s.level == level);

  /// Verbatim transcript of a level's most recent answer, or null when the
  /// level has no substantive turn. The trim used by [hasSubstantiveTurn]
  /// is evaluation-only (I-1); the stored text stays untouched (I-3).
  String? verbatimAt(ProbeLevel level) {
    for (final t in turns.reversed) {
      if (t.level == level) return t.answerVerbatim;
    }
    return null;
  }

  bool hasSubstantiveTurn(ProbeLevel level) {
    final v = verbatimAt(level);
    return v != null && v.trim().isNotEmpty;
  }

  Map<String, Object?> toJson() => {
    'session_id': sessionId,
    'mode': mode.wireValue,
    'topic': topic,
    'started_at': startedAt.toIso8601String(),
    'current_level': currentLevel.wireValue,
    'pending_question': pendingQuestion,
    'pending_question_source': pendingQuestionSource.wireValue,
    'level_turn_index': levelTurnIndex,
    'turns': turns.map((t) => t.toJson()).toList(),
    'skips': skips.map((s) => s.toJson()).toList(),
    'persona_discards': personaDiscards.map((d) => d.toJson()).toList(),
  };

  static ProbeChainState fromJson(Map<String, dynamic> json) {
    final started = json['started_at'];
    final state = ProbeChainState(
      sessionId: json['session_id'] as String? ?? '',
      mode: ElicitationMode.values.firstWhere(
        (m) => m.wireValue == (json['mode'] as String? ?? ''),
        orElse: () => ElicitationMode.freeRecall,
      ),
      topic: json['topic'] as String? ?? '',
      startedAt:
          started is String ? DateTime.tryParse(started)?.toUtc() : null,
    );
    state.currentLevel =
        ProbeLevel.fromWire(json['current_level'] as String? ?? 'surface');
    state.pendingQuestion = json['pending_question'] as String? ?? '';
    state.pendingQuestionSource = QuestionSource.values.firstWhere(
      (s) => s.wireValue == (json['pending_question_source'] as String? ?? ''),
      orElse: () => QuestionSource.staticBank,
    );
    state.levelTurnIndex = (json['level_turn_index'] as num?)?.toInt() ?? 0;
    final turns = json['turns'];
    final skips = json['skips'];
    final discards = json['persona_discards'];
    if (turns is List) {
      state.turns.addAll(turns.whereType<Map>().map(
            (t) => ProbeTurn.fromJson(Map<String, dynamic>.from(t)),
          ));
    }
    if (skips is List) {
      state.skips.addAll(skips.whereType<Map>().map(
            (s) => SkipEvent.fromJson(Map<String, dynamic>.from(s)),
          ));
    }
    if (discards is List) {
      state.personaDiscards.addAll(discards.whereType<Map>().map(
            (d) => PersonaDiscard.fromJson(Map<String, dynamic>.from(d)),
          ));
    }
    return state;
  }
}

/// Persona lock (design §4). The interviewer never breaks character —
/// never mentions being a model, never meta-comments, never refuses in
/// assistant-speak. Detection is deliberately heuristic and deterministic:
/// a small list of break-character markers. False negatives are fine (the
/// chain still works with a bland question); false positives just cost one
/// retry before falling back to the static bank.
class PersonaGuard {
  const PersonaGuard._();

  /// Lowercase markers that mean "the model broke character".
  static const List<String> breakMarkers = [
    'as an ai',
    'as a language model',
    "i'm an ai",
    'i am an ai',
    "i'm a language model",
    'i am a language model',
    'language model',
    'ai assistant',
    'ai model',
    'virtual assistant',
    'i cannot assist',
    "i can't assist",
    'i cannot help with',
    "i can't help with",
    'i am not able to',
    "i'm not able to",
    'i apologize',
    "i'm sorry, i can't",
    'my training data',
    'my knowledge cutoff',
    'system prompt',
    'i am a bot',
    "i'm a bot",
    'this is a simulation',
  ];

  /// Returns null when the proposal is in character; otherwise a short
  /// reason for the audit trail. Empty proposals fail trivially — the
  /// chain never asks a blank question.
  static String? validate(String proposal) {
    final text = proposal.trim();
    if (text.isEmpty) return 'empty proposal';
    if (text.length > 480) return 'proposal unreasonably long';
    final lower = text.toLowerCase();
    for (final marker in breakMarkers) {
      if (lower.contains(marker)) return 'break-character marker: "$marker"';
    }
    return null;
  }
}

/// A question ready for the Bench to display, with full provenance.
class ProbeQuestion {
  const ProbeQuestion({
    required this.level,
    required this.text,
    required this.source,
  });

  final ProbeLevel level;
  final String text;
  final QuestionSource source;
}

/// Result of [ProbeChainEngine.next]: what happened to the answer, plus a
/// pointer to the question now pending in state (the Bench reads
/// state.pendingQuestion, which is always the canonical on-screen text).
class ProbeStepResult {
  const ProbeStepResult({
    required this.escalated,
    this.pendingPersonaRetry = false,
  });

  /// True when the answer (or skip) advanced the chain; false means the
  /// chain re-asked in place (blank answer, I-1).
  final bool escalated;

  /// When a model proposal was discarded AND no valid retry proposal was
  /// supplied, this flag tells the Bench it may retry the model once
  /// before the static fallback stands. The question carried in
  /// state.pendingQuestion is already a valid bank fallback, so the flow
  /// never stalls on this flag.
  final bool pendingPersonaRetry;
}

/// Closure eligibility report (I-2 + I-5). The engine reports; only Dave
/// closes (via the registry gate, 5-b-2). The Dave-override path for
/// below-L4 chains is a gate_log event at the registry — it never routes
/// through this report.
class ClosureReport {
  const ClosureReport({
    required this.selfCloseEligible,
    required this.reason,
    required this.reachedContrast,
    required this.poisonedBySkip,
  });

  final bool selfCloseEligible;
  final String reason;
  final bool reachedContrast;
  final bool poisonedBySkip;
}

/// The chain contract. See [DefaultProbeChainEngine] for the reference
/// implementation; the abstraction exists so 5-b-3 tests can swap the
/// wording layer without touching state logic.
abstract class ProbeChainEngine {
  ProbeChainState start({
    required ElicitationMode mode,
    required String topic,
    String? modelProposal,
    String? retryProposal,
  });

  ProbeStepResult next(
    ProbeChainState state,
    String answer, {
    String? modelProposal,
    String? retryProposal,
  });

  void skip(
    ProbeChainState state,
    String reason, {
    String? modelProposal,
    String? retryProposal,
  });

  ClosureReport closureStatus(ProbeChainState state);

  List<RecordCandidate> mint(ProbeChainState state);
}

/// Reference implementation. Deterministic given (state, answer,
/// proposals): same inputs, same outputs, model offline safe.
class DefaultProbeChainEngine implements ProbeChainEngine {
  DefaultProbeChainEngine({
    QuestionBank? questionBank,
    String Function()? idFactory,
    DateTime Function()? clock,
  })  : _bank = questionBank ?? const QuestionBank(),
        _idFactory = idFactory,
        _clock = clock;

  final QuestionBank _bank;
  final String Function()? _idFactory;
  final DateTime Function()? _clock;

  String _newId() => _idFactory?.call() ??
      'rc-${DateTime.now().toUtc().microsecondsSinceEpoch.toRadixString(36)}';

  DateTime _now() => (_clock?.call() ?? DateTime.now()).toUtc();

  @override
  ProbeChainState start({
    required ElicitationMode mode,
    required String topic,
    String? modelProposal,
    String? retryProposal,
  }) {
    final trimmedTopic = topic.trim();
    if (trimmedTopic.isEmpty) {
      throw ArgumentError.value(topic, 'topic', 'chain needs a topic');
    }
    final state = ProbeChainState(
      sessionId: _newId(),
      mode: mode,
      topic: trimmedTopic,
      startedAt: _now(),
    );
    _resolveWording(
      state,
      ProbeLevel.surface,
      modelProposal: modelProposal,
      retryProposal: retryProposal,
    );
    return state;
  }

  /// Records [answer] at the current level per I-1, then resolves the
  /// question for the next step.
  ///
  /// Substantive answer (non-blank after trim — trim is evaluation-only,
  /// the stored text is verbatim): record, escalate one level, ask the
  /// next question. Blank: record nothing, stay at the level, return a
  /// re-ask. The chain can attempt every level; it can never force past
  /// silence — that is what [skip] is for.
  ///
  /// Wording resolution per design §4: [modelProposal] if in character,
  /// else [retryProposal] if supplied and in character, else the static
  /// bank. Every discarded proposal lands in state.personaDiscards.
  @override
  ProbeStepResult next(
    ProbeChainState state,
    String answer, {
    String? modelProposal,
    String? retryProposal,
  }) {
    final substantive = answer.trim().isNotEmpty;
    final level = state.currentLevel;

    if (!substantive) {
      // I-1: no substance, no escalation. Re-ask in place.
      state.levelTurnIndex += 1;
      state.pendingQuestionSource = QuestionSource.reask;
      state.pendingQuestion = _bank.questionFor(
        level,
        topic: state.topic,
        turnIndex: state.levelTurnIndex,
        isReask: true,
      );
      return const ProbeStepResult(escalated: false);
    }

    state.turns.add(ProbeTurn(
      level: level,
      question: state.pendingQuestion,
      answerVerbatim: answer, // I-3: verbatim, untouched.
      at: _now(),
      questionSource: state.pendingQuestionSource,
    ));

    if (level == ProbeLevel.meaning) {
      return const ProbeStepResult(escalated: true);
    }

    final nextLevel = ProbeLevel.values[level.index + 1];
    state.currentLevel = nextLevel;
    state.levelTurnIndex = 0;
    final question = _resolveWording(
      state,
      nextLevel,
      modelProposal: modelProposal,
      retryProposal: retryProposal,
    );
    return ProbeStepResult(
      escalated: true,
      pendingPersonaRetry: question == null,
    );
  }

  /// Explicit "I don't remember". Records the skip at the current level
  /// and advances (I-1 escalation path). At L2-L4 this poisons
  /// self-closure (I-5); the poison is visible in state.skips forever.
  /// The next level's question is resolved from the bank unless the caller
  /// supplies model wording (the Bench passes proposals here the same way
  /// it does for next).
  @override
  void skip(
    ProbeChainState state,
    String reason, {
    String? modelProposal,
    String? retryProposal,
  }) {
    final level = state.currentLevel;
    state.skips.add(SkipEvent(level: level, reason: reason, at: _now()));
    if (level != ProbeLevel.meaning) {
      final nextLevel = ProbeLevel.values[level.index + 1];
      state.currentLevel = nextLevel;
      state.levelTurnIndex = 0;
      _resolveWording(
        state,
        nextLevel,
        modelProposal: modelProposal,
        retryProposal: retryProposal,
      );
    }
  }

  /// I-2 + I-5: self-closure is eligible only when the chain has a
  /// substantive contrast (L4) turn AND no skips at sensory/source/
  /// contrast. Meaning (L5) is the natural end, not the requirement —
  /// an honest chain that lands at contrast with substance is closable;
  /// a chain that reached meaning past an L2 skip is not.
  @override
  ClosureReport closureStatus(ProbeChainState state) {
    final reachedContrast = state.hasSubstantiveTurn(ProbeLevel.contrast);
    final poisoned = state.hasSkipAt(ProbeLevel.sensory) ||
        state.hasSkipAt(ProbeLevel.source) ||
        state.hasSkipAt(ProbeLevel.contrast);
    final eligible = reachedContrast && !poisoned;
    return ClosureReport(
      selfCloseEligible: eligible,
      reason: !reachedContrast
          ? 'below contrast (L4): the code path cannot close (I-2); '
              'only Dave can, manually, stamped in gate_log'
          : poisoned
              ? 'skip at L2-L4 poisons closure even at meaning (I-5)'
              : 'contrast anchored, no poisoned skips; Dave still closes '
                  'via the gate — the engine never closes (I-4)',
      reachedContrast: reachedContrast,
      poisonedBySkip: poisoned,
    );
  }

  /// Mint per the standing table. Candidates only, forever (I-4).
  ///
  ///   surface substantive            -> event @ 1.0 (verbatim)
  ///   contrast substantive, no L2-4 skip
  ///     + meaning substantive        -> judgment @ 1.0 (carries chain)
  ///     else                         -> judgment @ 0.8 (carries chain)
  ///   any L2-L4 skip                 -> event + memory_candidate @ 0.3
  ///   surface blank/skip             -> nothing (an empty chain mints
  ///                                     nothing; the session row still
  ///                                     exists at the registry layer)
  ///
  /// Claims are never minted here (need >=2 sources; registry's job).
  @override
  List<RecordCandidate> mint(ProbeChainState state) {
    if (!state.hasSubstantiveTurn(ProbeLevel.surface)) return const [];

    final probeChainLabels = <String>[
      for (final t in state.turns) t.level.wireValue,
      for (final s in state.skips) '${s.level.wireValue}.skip',
    ];
    final locator = '@session:${state.sessionId}';
    final now = _now();

    final surfaceText = state.verbatimAt(ProbeLevel.surface)!;
    final event = RecordCandidate(
      recordId: _newId(),
      recordType: RecordType.event,
      canonicalText: surfaceText,
      confidence: RecordCandidate.eventConfidence,
      probeChain: List.unmodifiable(probeChainLabels),
      sources: [locator],
      createdAt: now,
    );

    if (state.hasSkipAt(ProbeLevel.sensory) ||
        state.hasSkipAt(ProbeLevel.source) ||
        state.hasSkipAt(ProbeLevel.contrast)) {
      return [
        event,
        RecordCandidate(
          recordId: _newId(),
          recordType: RecordType.memoryCandidate,
          canonicalText: surfaceText,
          confidence: RecordCandidate.memoryCandidateConfidence,
          probeChain: List.unmodifiable(probeChainLabels),
          sources: [locator],
          createdAt: now,
        ),
      ];
    }

    if (state.hasSubstantiveTurn(ProbeLevel.contrast)) {
      final full = state.hasSubstantiveTurn(ProbeLevel.meaning);
      return [
        event,
        RecordCandidate(
          recordId: _newId(),
          recordType: RecordType.judgment,
          canonicalText: surfaceText,
          confidence: full
              ? RecordCandidate.judgmentConfidenceFull
              : RecordCandidate.judgmentConfidenceContrast,
          probeChain: List.unmodifiable(probeChainLabels),
          sources: [locator],
          createdAt: now,
        ),
      ];
    }

    return [event];
  }

  // --- wording internals -------------------------------------------------

  /// Resolves wording for [level] and commits it to state.pendingQuestion.
  /// Returns null exactly when at least one proposal was discarded and no
  /// valid replacement stood — the Bench's cue for a single model retry.
  ProbeQuestion? _resolveWording(
    ProbeChainState state,
    ProbeLevel level, {
    String? modelProposal,
    String? retryProposal,
  }) {
    var discarded = false;

    if (modelProposal != null) {
      final fail = PersonaGuard.validate(modelProposal);
      if (fail == null) {
        return _commit(state, level, modelProposal, QuestionSource.modelProposal);
      }
      discarded = true;
      state.personaDiscards.add(
        PersonaDiscard(level: level, reason: fail, at: _now()),
      );
      if (retryProposal != null) {
        final fail2 = PersonaGuard.validate(retryProposal);
        if (fail2 == null) {
          return _commit(
              state, level, retryProposal, QuestionSource.retryProposal);
        }
        state.personaDiscards.add(
          PersonaDiscard(level: level, reason: fail2, at: _now()),
        );
      }
    }

    final text = _bank.questionFor(
      level,
      topic: state.topic,
      turnIndex: state.levelTurnIndex,
    );
    _commit(state, level, text, QuestionSource.staticBank);
    return discarded ? null : ProbeQuestion(level: level, text: text, source: QuestionSource.staticBank);
  }

  ProbeQuestion _commit(
    ProbeChainState state,
    ProbeLevel level,
    String text,
    QuestionSource source,
  ) {
    state.pendingQuestion = text;
    state.pendingQuestionSource = source;
    return ProbeQuestion(level: level, text: text, source: source);
  }
}
