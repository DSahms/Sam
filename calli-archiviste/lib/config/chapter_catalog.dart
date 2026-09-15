import 'package:flutter_dotenv/flutter_dotenv.dart';

import 'constants.dart';
import 'recovery_constants.dart';

enum ChapterSet { memoir, recovery }

/// Selects memoir vs recovery chapter metadata from .env (`CHAPTER_SET=recovery`).
class ChapterCatalog {

  // Extraction guard (2026-09-15): the host app may not have loaded a .env
  // yet; missing configuration then falls back to the getter defaults
  // instead of throwing.
  static String? _env(String key) {
    try {
      return dotenv.env[key];
    } catch (_) {
      return null;
    }
  }

  static ChapterSet get activeSet {
    final raw = _env('CHAPTER_SET')?.trim().toLowerCase();
    if (raw == 'recovery') {
      return ChapterSet.recovery;
    }
    return ChapterSet.memoir;
  }

  static bool get isRecovery => activeSet == ChapterSet.recovery;

  static const List<LifeChapter> memoirChapterOrder = [
    LifeChapter.childhood,
    LifeChapter.adolescence,
    LifeChapter.family,
    LifeChapter.education,
    LifeChapter.career,
    LifeChapter.love,
    LifeChapter.parenthood,
    LifeChapter.loss,
    LifeChapter.turning,
    LifeChapter.lessons,
    LifeChapter.legacy,
  ];

  static List<LifeChapter> get chapters =>
      isRecovery ? RecoveryChapterConstants.chapterOrder : memoirChapterOrder;

  static String title(LifeChapter chapter) =>
      _titles[chapter] ?? 'This Chapter';

  static String subtitle(LifeChapter chapter) =>
      _subtitles[chapter] ?? '';

  static String icon(LifeChapter chapter) => _icons[chapter] ?? '📖';

  static int sortOrder(LifeChapter chapter) =>
      _sortOrder[chapter] ?? 0;

  static List<String> seedQuestions(LifeChapter chapter) =>
      _seeds[chapter] ?? const [];

  static Map<LifeChapter, String> get _titles =>
      isRecovery ? RecoveryChapterConstants.chapterTitles : ChapterConstants.chapterTitles;

  static Map<LifeChapter, String> get _subtitles => isRecovery
      ? RecoveryChapterConstants.chapterSubtitles
      : ChapterConstants.chapterSubtitles;

  static Map<LifeChapter, String> get _icons =>
      isRecovery ? RecoveryChapterConstants.chapterIcons : ChapterConstants.chapterIcons;

  static Map<LifeChapter, int> get _sortOrder => isRecovery
      ? RecoveryChapterConstants.chapterSortOrder
      : ChapterConstants.chapterSortOrder;

  static Map<LifeChapter, List<String>> get _seeds => isRecovery
      ? RecoveryChapterConstants.seedQuestions
      : ChapterConstants.seedQuestions;
}
