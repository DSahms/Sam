import 'package:uuid/uuid.dart';

/// The role of a message sender
enum MessageRole {
  interviewer,
  user,
}

/// A single message in an interview conversation
class Message {
  final String id;
  final MessageRole role;
  final String content;
  final DateTime timestamp;
  final Map<String, dynamic>? metadata;

  Message({
    String? id,
    required this.role,
    required this.content,
    DateTime? timestamp,
    this.metadata,
  })  : id = id ?? const Uuid().v4(),
        timestamp = timestamp ?? DateTime.now();

  /// Convert to JSON for storage
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'role': role.toString(),
      'content': content,
      'timestamp': timestamp.toIso8601String(),
      'metadata': metadata,
    };
  }

  /// Create from JSON
  factory Message.fromJson(Map<String, dynamic> json) {
    final roleStr = json['role'] as String;
    final role = MessageRole.values.firstWhere(
      (e) => e.toString() == roleStr,
      orElse: () => MessageRole.user,
    );

    return Message(
      id: json['id'] as String? ?? const Uuid().v4(),
      role: role,
      content: json['content'] as String? ?? '',
      timestamp: json['timestamp'] != null
          ? DateTime.parse(json['timestamp'] as String)
          : DateTime.now(),
      metadata: json['metadata'] as Map<String, dynamic>?,
    );
  }

  /// Create a copy with optional parameter overrides
  Message copyWith({
    String? id,
    MessageRole? role,
    String? content,
    DateTime? timestamp,
    Map<String, dynamic>? metadata,
  }) {
    return Message(
      id: id ?? this.id,
      role: role ?? this.role,
      content: content ?? this.content,
      timestamp: timestamp ?? this.timestamp,
      metadata: metadata ?? this.metadata,
    );
  }
}