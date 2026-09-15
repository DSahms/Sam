import 'package:uuid/uuid.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/models/message.dart';

/// An interview session for a specific chapter
class InterviewSession {
  final String id;
  final LifeChapter chapter;
  final DateTime startedAt;
  final DateTime? endedAt;
  final List<Message> messages;
  final String? summary;
  final bool isComplete;

  InterviewSession({
    String? id,
    required this.chapter,
    DateTime? startedAt,
    this.endedAt,
    List<Message>? messages,
    this.summary,
    this.isComplete = false,
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
      'chapter': chapter.toString(),
      'startedAt': startedAt.toIso8601String(),
      'endedAt': endedAt?.toIso8601String(),
      'messages': messages.map((m) => m.toJson()).toList(),
      'summary': summary,
      'isComplete': isComplete,
    };
  }

  /// Create from JSON
  factory InterviewSession.fromJson(Map<String, dynamic> json) {
    final chapterStr = json['chapter'] as String;
    final chapter = LifeChapter.values.firstWhere(
      (e) => e.toString() == chapterStr,
      orElse: () => LifeChapter.childhood,
    );

    final messagesList = (json['messages'] as List<dynamic>?)
            ?.map((m) => Message.fromJson(m as Map<String, dynamic>))
            .toList() ??
        [];

    return InterviewSession(
      id: json['id'] as String? ?? const Uuid().v4(),
      chapter: chapter,
      startedAt: json['startedAt'] != null
          ? DateTime.parse(json['startedAt'] as String)
          : DateTime.now(),
      endedAt: json['endedAt'] != null
          ? DateTime.parse(json['endedAt'] as String)
          : null,
      messages: messagesList,
      summary: json['summary'] as String?,
      isComplete: json['isComplete'] as bool? ?? false,
    );
  }

  /// Create a copy with optional parameter overrides
  InterviewSession copyWith({
    String? id,
    LifeChapter? chapter,
    DateTime? startedAt,
    DateTime? endedAt,
    List<Message>? messages,
    String? summary,
    bool? isComplete,
  }) {
    return InterviewSession(
      id: id ?? this.id,
      chapter: chapter ?? this.chapter,
      startedAt: startedAt ?? this.startedAt,
      endedAt: endedAt ?? this.endedAt,
      messages: messages ?? this.messages,
      summary: summary ?? this.summary,
      isComplete: isComplete ?? this.isComplete,
    );
  }
}
