import 'package:uuid/uuid.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/models/intake_phase.dart';

/// An intake session for personal knowledge gathering
class IntakeSession {
  final String id;
  final IntakePhase phase;
  final DateTime startedAt;
  final DateTime? endedAt;
  final List<Message> messages;
  final String? summary;
  final bool isComplete;
  final String? artifactId; // Reference to Media Archive artifact

  IntakeSession({
    String? id,
    required this.phase,
    DateTime? startedAt,
    this.endedAt,
    List<Message>? messages,
    this.summary,
    this.isComplete = false,
    this.artifactId,
  })  : id = id ?? const Uuid().v4(),
        startedAt = startedAt ?? DateTime.now(),
        messages = messages ?? [];

  /// Get the duration of the session
  Duration get duration {
    final end = endedAt ?? DateTime.now();
    return end.difference(startedAt);
  }

  /// Get the number of user answers/messages
  int get answerCount {
    return messages.where((m) => m.role == MessageRole.user).length;
  }

  /// Convert to JSON for storage
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'phase': phase.toString(),
      'startedAt': startedAt.toIso8601String(),
      'endedAt': endedAt?.toIso8601String(),
      'messages': messages.map((m) => m.toJson()).toList(),
      'summary': summary,
      'isComplete': isComplete,
      'artifactId': artifactId,
    };
  }

  /// Create from JSON
  factory IntakeSession.fromJson(Map<String, dynamic> json) {
    final phaseStr = json['phase'] as String;
    final phase = IntakePhase.values.firstWhere(
      (e) => e.toString() == phaseStr,
      orElse: () => IntakePhase.artifactIdentification,
    );

    final messagesList = (json['messages'] as List<dynamic>?)
            ?.map((m) => Message.fromJson(m as Map<String, dynamic>))
            .toList() ??
        [];

    return IntakeSession(
      id: json['id'] as String? ?? const Uuid().v4(),
      phase: phase,
      startedAt: json['startedAt'] != null
          ? DateTime.parse(json['startedAt'] as String)
          : DateTime.now(),
      endedAt: json['endedAt'] != null
          ? DateTime.parse(json['endedAt'] as String)
          : null,
      messages: messagesList,
      summary: json['summary'] as String?,
      isComplete: json['isComplete'] as bool? ?? false,
      artifactId: json['artifactId'] as String?,
    );
  }

  /// Create a copy with optional parameter overrides
  IntakeSession copyWith({
    String? id,
    IntakePhase? phase,
    DateTime? startedAt,
    DateTime? endedAt,
    List<Message>? messages,
    String? summary,
    bool? isComplete,
    String? artifactId,
  }) {
    return IntakeSession(
      id: id ?? this.id,
      phase: phase ?? this.phase,
      startedAt: startedAt ?? this.startedAt,
      endedAt: endedAt ?? this.endedAt,
      messages: messages ?? this.messages,
      summary: summary ?? this.summary,
      isComplete: isComplete ?? this.isComplete,
      artifactId: artifactId ?? this.artifactId,
    );
  }
}