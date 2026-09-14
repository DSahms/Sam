/// Represents the phase of an intake interview.
/// Unlike StoryKeeper's LifeChapter (memoir-specific), this is generic
/// and can be extended by the application layer (Sam) as needed.
enum IntakePhase {
  /// Initial artifact identification and context gathering
  artifactIdentification,

  /// Gathering who/what/when/where facts
  factGathering,

  /// Capturing personal meaning and context
  meaningCapture,

  /// Review and confirmation
  review,

  /// Session complete, ready for PKC submission
  complete,
}

/// Extension for display names
extension IntakePhaseExtension on IntakePhase {
  String get displayName {
    switch (this) {
      case IntakePhase.artifactIdentification:
        return 'Artifact Identification';
      case IntakePhase.factGathering:
        return 'Fact Gathering';
      case IntakePhase.meaningCapture:
        return 'Meaning Capture';
      case IntakePhase.review:
        return 'Review';
      case IntakePhase.complete:
        return 'Complete';
    }
  }

  int get sortOrder {
    switch (this) {
      case IntakePhase.artifactIdentification:
        return 0;
      case IntakePhase.factGathering:
        return 1;
      case IntakePhase.meaningCapture:
        return 2;
      case IntakePhase.review:
        return 3;
      case IntakePhase.complete:
        return 4;
    }
  }
}