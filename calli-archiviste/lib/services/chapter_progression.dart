import 'package:calli_archiviste/models/intake_phase.dart';

/// Calli intake progression rules for managing interview flow
class IntakeProgression {
  /// Default number of questions per intake phase session
  static const int defaultQuestionsPerPhaseSession = 12;

  /// Visible warning threshold within a phase session
  static const int warnQuestionsPerPhase = 15;

  /// Hard pause threshold — never silently continue past this in one phase
  static const int maxQuestionsPerPhase = 25;

  /// Total safety cap for one intake invocation (all phases)
  static const int intakeSafetyMaxQuestions = 100;

  static IntakePhase? nextPhase(IntakePhase current) {
    final order = IntakePhase.values;
    final index = order.indexOf(current);
    if (index < 0 || index >= order.length - 1) return null;
    return order[index + 1];
  }

  static int phaseNumber(IntakePhase phase) {
    final index = IntakePhase.values.indexOf(phase);
    return index < 0 ? 0 : index + 1;
  }

  static bool isFinalPhase(IntakePhase phase) {
    final order = IntakePhase.values;
    return order.isNotEmpty && phase == order.last;
  }
}