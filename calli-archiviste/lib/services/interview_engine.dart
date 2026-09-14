import 'package:calli_archiviste/models/intake_phase.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/models/session.dart';

/// The Calli intake interview engine interface.
///
/// MIGRATION NOTE (2026-09-14):
/// The authoritative donor implementation lives in
/// `D:\dev\StoryKeeper-Local-Writer\lib\services\interview_engine.dart`
/// (protected donor — see `docs/migration/calli-migration-plan.md` and
/// `NAMING.md` at the repository root).
///
/// Per the master architecture document (§17), there must be EXACTLY ONE
/// authoritative interview engine. The real engine must be EXTRACTED from
/// the donor and adapted here — it must not be rewritten from scratch, and
/// no second engine may be created.
///
/// This file provides the Calli-native interface plus a deterministic
/// offline stub so that the package analyzes, tests can run, and the UI can
/// be exercised before the real extraction lands.
abstract interface class InterviewEngine {
  /// Generate the next interview question for the current intake session.
  ///
  /// Implementations must be deterministic-in-testable, must never write to
  /// durable storage (SessionStorage owns persistence), and must respect the
  /// session's current [IntakePhase].
  Future<String> generateNextQuestion({required IntakeSession session});
}

/// Deterministic offline [InterviewEngine] stub.
///
/// Picks a phase-appropriate question from fixed lists, advancing one
/// question per interviewer turn already present in the session. Contains no
/// LLM calls, no I/O, and no randomness, so it is safe for widget tests and
/// for exercising the intake UI before the real engine is extracted.
///
/// STUB — replace via the donor extraction described above. Do not grow
/// this class into a second engine.
class StubInterviewEngine implements InterviewEngine {
  final Map<IntakePhase, List<String>> _questionsByPhase;

  StubInterviewEngine({Map<IntakePhase, List<String>>? questionsByPhase})
      : _questionsByPhase = questionsByPhase ?? _defaultQuestions;

  static const Map<IntakePhase, List<String>> _defaultQuestions = {
    IntakePhase.artifactIdentification: [
      'What artifact are we adding today?',
      'Where did this come from, and when did it enter your hands?',
      'Is anything physically notable about it that we should record?',
    ],
    IntakePhase.factGathering: [
      'Who is connected to this?',
      'When and where did this take place?',
      'Who else was involved or present?',
      'What happened before or after this?',
    ],
    IntakePhase.meaningCapture: [
      'Why does this matter to you?',
      'What should be remembered about this?',
      'Is there a source that supports what you have told me?',
    ],
    IntakePhase.review: [
      'Here is what I have captured. What needs correcting or adding?',
    ],
    IntakePhase.complete: [],
  };

  @override
  Future<String> generateNextQuestion({required IntakeSession session}) async {
    if (session.isComplete || session.phase == IntakePhase.complete) {
      return 'This session is complete and ready for review.';
    }

    final questions = _questionsByPhase[session.phase] ??
        _questionsByPhase[IntakePhase.artifactIdentification]!;

    // Advance one question per interviewer turn already recorded.
    final asked = session.messages
        .where((m) => m.role == MessageRole.interviewer)
        .length;

    if (asked >= questions.length) {
      return 'Anything else about this, or shall we move to the next part?';
    }
    return questions[asked];
  }
}
