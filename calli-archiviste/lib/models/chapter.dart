import 'package:calli_archiviste/config/chapter_catalog.dart';
import 'package:calli_archiviste/config/constants.dart';

/// Metadata for a life chapter
class ChapterMetadata {
  final LifeChapter chapter;
  final String title;
  final String subtitle;
  final String icon;
  final int sortOrder;
  final List<String> seedQuestions;
  final int completedSessions;
  final int totalMessages;

  ChapterMetadata({
    required this.chapter,
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.sortOrder,
    required this.seedQuestions,
    this.completedSessions = 0,
    this.totalMessages = 0,
  });

  /// Calculate progress as a ratio (0.0 to 1.0)
  /// Based on completedSessions out of 5
  double get progress {
    return (completedSessions / 5).clamp(0.0, 1.0);
  }

  /// Create a copy with optional parameter overrides
  ChapterMetadata copyWith({
    LifeChapter? chapter,
    String? title,
    String? subtitle,
    String? icon,
    int? sortOrder,
    List<String>? seedQuestions,
    int? completedSessions,
    int? totalMessages,
  }) {
    return ChapterMetadata(
      chapter: chapter ?? this.chapter,
      title: title ?? this.title,
      subtitle: subtitle ?? this.subtitle,
      icon: icon ?? this.icon,
      sortOrder: sortOrder ?? this.sortOrder,
      seedQuestions: seedQuestions ?? this.seedQuestions,
      completedSessions: completedSessions ?? this.completedSessions,
      totalMessages: totalMessages ?? this.totalMessages,
    );
  }

  /// Convert to JSON for storage
  Map<String, dynamic> toJson() {
    return {
      'chapter': chapter.toString(),
      'title': title,
      'subtitle': subtitle,
      'icon': icon,
      'sortOrder': sortOrder,
      'seedQuestions': seedQuestions,
      'completedSessions': completedSessions,
      'totalMessages': totalMessages,
    };
  }

  /// Create from JSON
  factory ChapterMetadata.fromJson(Map<String, dynamic> json) {
    final chapterStr = json['chapter'] as String;
    final chapter = LifeChapter.values.firstWhere(
      (e) => e.toString() == chapterStr,
      orElse: () => LifeChapter.childhood,
    );

    return ChapterMetadata(
      chapter: chapter,
      title: json['title'] as String? ?? '',
      subtitle: json['subtitle'] as String? ?? '',
      icon: json['icon'] as String? ?? '',
      sortOrder: json['sortOrder'] as int? ?? 0,
      seedQuestions: List<String>.from(
        (json['seedQuestions'] as List<dynamic>?) ?? [],
      ),
      completedSessions: json['completedSessions'] as int? ?? 0,
      totalMessages: json['totalMessages'] as int? ?? 0,
    );
  }

  /// Create metadata from chapter enum using constants
  factory ChapterMetadata.fromChapter(LifeChapter chapter) {
    return ChapterMetadata(
      chapter: chapter,
      title: ChapterCatalog.title(chapter),
      subtitle: ChapterCatalog.subtitle(chapter),
      icon: ChapterCatalog.icon(chapter),
      sortOrder: ChapterCatalog.sortOrder(chapter),
      seedQuestions: ChapterCatalog.seedQuestions(chapter),
    );
  }
}
