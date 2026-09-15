import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:hive_flutter/hive_flutter.dart';
import 'package:calli_archiviste/config/chapter_catalog.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/models/chapter.dart';
import 'package:calli_archiviste/models/family_member.dart';
import 'package:calli_archiviste/models/photo_metadata.dart';
import 'package:calli_archiviste/models/interview_session.dart';
import 'package:calli_archiviste/services/context_compressor.dart';

/// Service for persisting sessions and chapter metadata using Hive
class SessionStorage {
  static const String _sessionsBox = 'sessions';
  static const String _chaptersBox = 'chapters';
  static const String _familyBox = 'family';
  static const String _photosBox = 'photos';

  static const String _profileBox = 'profile';
  static const String _narrativesBox = 'narratives';
  static const String _bookConfigBox = 'bookConfig';

  late Box<String> _sessionBox;
  late Box<String> _chapterBox;
  late Box<String> _familyBoxInstance;
  late Box<String> _photosBoxInstance;
  late Box<String> _profileBoxInstance;
  late Box<String> _narrativesBoxInstance;
  late Box<String> _bookConfigBoxInstance;

  /// Initialize Hive and open all boxes
  Future<void> initialize() async {
    _sessionBox = await Hive.openBox<String>(_sessionsBox);
    _chapterBox = await Hive.openBox<String>(_chaptersBox);
    _familyBoxInstance = await Hive.openBox<String>(_familyBox);
    _photosBoxInstance = await Hive.openBox<String>(_photosBox);
    _profileBoxInstance = await Hive.openBox<String>(_profileBox);
    _narrativesBoxInstance = await Hive.openBox<String>(_narrativesBox);
    _bookConfigBoxInstance = await Hive.openBox<String>(_bookConfigBox);
  }

  Box<String> _openBox(String name) => Hive.box<String>(name);

  Box<String> get _familyMemberBox => Hive.box<String>(_familyBox);

  /// Save a family member.
  Future<void> saveFamilyMember(FamilyMember member) async {
    await _familyMemberBox.put(
      member.id, jsonEncode(member.toJson()));
  }

  /// Get all family members.
  Future<List<FamilyMember>> getAllFamilyMembers() async {
    final members = <FamilyMember>[];
    for (final key in _familyMemberBox.keys) {
      final json = _familyMemberBox.get(key);
      if (json != null) {
        members.add(FamilyMember.fromJson(
          jsonDecode(json) as Map<String, dynamic>));
      }
    }
    return members;
  }

  /// Delete a family member.
  Future<void> deleteFamilyMember(String memberId) async {
    await _familyMemberBox.delete(memberId);
  }

  /// Get family members relevant to a specific chapter.
  /// For 'family' chapter, return all members.
  /// For 'childhood', return parents and grandparents.
  /// For 'parenthood', return children and spouse.
  /// For other chapters, return all members (the AI will
  /// decide what's relevant).
  Future<List<FamilyMember>> getFamilyMembersForChapter(
    LifeChapter chapter
  ) async {
    return getAllFamilyMembers();
  }

  /// Save a photo.
  Future<void> savePhoto(PhotoMetadata photo) async {
    await _openBox(_photosBox).put(photo.id, jsonEncode(photo.toJson()));
  }

  /// Get photos for a specific chapter.
  Future<List<PhotoMetadata>> getPhotosForChapter(LifeChapter chapter) async {
    final photos = <PhotoMetadata>[];
    for (final key in _openBox(_photosBox).keys) {
      final json = _openBox(_photosBox).get(key);
      if (json != null) {
        try {
          final photo = PhotoMetadata.fromJson(
            jsonDecode(json) as Map<String, dynamic>);
          if (photo.chapter == chapter) {
            photos.add(photo);
          }
        } catch (e) {
          continue;
        }
      }
    }
    return photos;
  }

  /// Delete a photo by ID.
  Future<void> deletePhoto(String photoId) async {
    await _openBox(_photosBox).delete(photoId);
  }

  /// Save a session
  Future<void> saveSession(InterviewSession session) async {
    debugPrint('STORAGE DEBUG: Saving session ${session.id} with ${session.messages.length} messages');
    await _openBox(_sessionsBox).put(session.id, jsonEncode(session.toJson()));
  }

  /// Get a session by ID
  Future<InterviewSession?> getSession(String sessionId) async {
    final data = _openBox(_sessionsBox).get(sessionId);
    if (data is String) {
      try {
        return InterviewSession.fromJson(
          jsonDecode(data) as Map<String, dynamic>,
        );
      } catch (e) {
        return null;
      }
    }
    return null;
  }

  /// Get all sessions for a specific chapter
  Future<List<InterviewSession>> getSessionsForChapter(
    LifeChapter chapter,
  ) async {
    final sessions = <InterviewSession>[];
    for (final key in _openBox(_sessionsBox).keys) {
      final sessionStr = _openBox(_sessionsBox).get(key);
      if (sessionStr != null) {
        try {
          final session = InterviewSession.fromJson(
            jsonDecode(sessionStr) as Map<String, dynamic>,
          );
          if (session.chapter == chapter) {
            sessions.add(session);
          }
        } catch (e) {
          continue;
        }
      }
    }
    return sessions;
  }

  /// Get the most recent active session (not completed)
  Future<InterviewSession?> getActiveSession() async {
    InterviewSession? activeSession;
    DateTime? mostRecent;

    for (final key in _openBox(_sessionsBox).keys) {
      final sessionStr = _openBox(_sessionsBox).get(key);
      if (sessionStr != null) {
        try {
          final session = InterviewSession.fromJson(
            jsonDecode(sessionStr) as Map<String, dynamic>,
          );
          if (!session.isComplete) {
            if (mostRecent == null || session.startedAt.isAfter(mostRecent)) {
              activeSession = session;
              mostRecent = session.startedAt;
            }
          }
        } catch (e) {
          continue;
        }
      }
    }
    return activeSession;
  }

  /// Get summaries of all sessions
  Future<List<Map<String, dynamic>>> getSessionSummaries() async {
    final summaries = <Map<String, dynamic>>[];

    for (final key in _openBox(_sessionsBox).keys) {
      final sessionStr = _openBox(_sessionsBox).get(key);
      if (sessionStr != null) {
        try {
          final session = InterviewSession.fromJson(
            jsonDecode(sessionStr) as Map<String, dynamic>,
          );
          summaries.add({
            'id': session.id,
            'chapter': session.chapter,
            'startedAt': session.startedAt,
            'endedAt': session.endedAt,
            'messageCount': session.messages.length,
            'answerCount': session.answerCount,
            'isComplete': session.isComplete,
            'summary': session.summary,
          });
        } catch (e) {
          continue;
        }
      }
    }

    return summaries;
  }

  /// Get metadata for a chapter
  Future<ChapterMetadata?> getChapterMetadata(LifeChapter chapter) async {
    final key = chapter.toString();
    final data = _openBox(_chaptersBox).get(key);

    if (data is String) {
      try {
        return ChapterMetadata.fromJson(
          jsonDecode(data) as Map<String, dynamic>,
        );
      } catch (e) {
        return null;
      }
    }

    // Return default metadata if not found
    return ChapterMetadata.fromChapter(chapter);
  }

  /// Save metadata for a chapter
  Future<void> saveChapterMetadata(ChapterMetadata metadata) async {
    final key = metadata.chapter.toString();
    await _openBox(_chaptersBox).put(key, jsonEncode(metadata.toJson()));
  }

  /// Get all chapter metadata
  Future<List<ChapterMetadata>> getAllChapterMetadata() async {
    final chapters = <ChapterMetadata>[];

    for (final chapter in ChapterCatalog.chapters) {
      final metadata = await getChapterMetadata(chapter);
      if (metadata != null) {
        chapters.add(metadata);
      }
    }

    // Sort by sort order
    chapters.sort((a, b) => a.sortOrder.compareTo(b.sortOrder));
    return chapters;
  }

  /// Clear interview content but keep onboarding profile.
  Future<void> clearAll() async {
    await _openBox(_sessionsBox).clear();
    await _openBox(_chaptersBox).clear();
    await _openBox(_familyBox).clear();
    await _openBox(_photosBox).clear();
    await _openBox(_narrativesBox).clear();
    await _openBox(_bookConfigBox).clear();
  }

  /// Return to a genuine first-run state, including profile and photo files.
  Future<void> resetAllData() async {
    await _deleteStoredPhotoFiles();
    await _openBox(_sessionsBox).clear();
    await _openBox(_chaptersBox).clear();
    await _openBox(_familyBox).clear();
    await _openBox(_photosBox).clear();
    await _openBox(_narrativesBox).clear();
    await _openBox(_bookConfigBox).clear();
    await _openBox(_profileBox).clear();
    debugPrint('ALL DATA RESET');
  }

  Future<void> _deleteStoredPhotoFiles() async {
    final photosBox = _openBox(_photosBox);
    for (final key in photosBox.keys.toList()) {
      final json = photosBox.get(key);
      if (json == null) {
        continue;
      }
      try {
        final photo = PhotoMetadata.fromJson(
          jsonDecode(json) as Map<String, dynamic>,
        );
        final file = File(photo.filePath);
        if (await file.exists()) {
          await file.delete();
        }
      } catch (_) {
        continue;
      }
    }
  }

  /// Save a generated narrative for a chapter.
  ///
  /// `sourceSignature` ties the cached narrative to the exact session content
  /// used to generate it. This prevents stale preview/PDF content from leaking
  /// when interview data changes.
  Future<void> saveNarrative(
    LifeChapter chapter,
    String narrative, {
    String? sourceSignature,
  }) async {
    if (sourceSignature == null || sourceSignature.isEmpty) {
      await _openBox(_narrativesBox).put(chapter.name, narrative);
      return;
    }
    final payload = jsonEncode({
      'version': 2,
      'narrative': narrative,
      'sourceSignature': sourceSignature,
      'updatedAt': DateTime.now().toIso8601String(),
    });
    await _openBox(_narrativesBox).put(chapter.name, payload);
  }

  /// Get cached narrative for a chapter (null if not found or empty).
  ///
  /// Backward-compatible with legacy plain-string entries.
  Future<String?> getNarrative(LifeChapter chapter) async {
    final value = _openBox(_narrativesBox).get(chapter.name);
    if (value == null || value.isEmpty) return null;
    if (value is! String) return null;
    try {
      final decoded = jsonDecode(value);
      if (decoded is Map<String, dynamic>) {
        final narrative = decoded['narrative'];
        if (narrative is String && narrative.isNotEmpty) {
          return narrative;
        }
      }
    } catch (_) {
      // Legacy plain-string narrative.
    }
    return value;
  }

  /// Read the source signature saved with a cached narrative.
  ///
  /// Returns null for legacy entries that predate signature tracking.
  Future<String?> getNarrativeSourceSignature(LifeChapter chapter) async {
    final value = _openBox(_narrativesBox).get(chapter.name);
    if (value == null || value is! String || value.isEmpty) return null;
    try {
      final decoded = jsonDecode(value);
      if (decoded is Map<String, dynamic>) {
        final signature = decoded['sourceSignature'];
        if (signature is String && signature.isNotEmpty) {
          return signature;
        }
      }
    } catch (_) {
      // Legacy plain-string narrative.
    }
    return null;
  }

  /// Save book configuration JSON
  Future<void> saveBookConfig(String configJson) async {
    await _openBox(_bookConfigBox).put('config', configJson);
  }

  /// Get saved book configuration JSON (null if not found)
  Future<String?> getBookConfig() async {
    return _openBox(_bookConfigBox).get('config');
  }

  /// Delete a session by ID
  Future<void> deleteSession(String sessionId) async {
    await _openBox(_sessionsBox).delete(sessionId);
    debugPrint('STORAGE DEBUG: Deleted session $sessionId');
  }

  /// Save user profile
  Future<void> saveUserProfile({
    required String firstName,
    String? middleName,
    required String lastName,
    required String preferredName,
    DateTime? dateOfBirth,
  }) async {
    final box = Hive.box<String>(_profileBox);
    await box.put('firstName', firstName);
    if (middleName != null && middleName.isNotEmpty) {
      await box.put('middleName', middleName);
    }
    await box.put('lastName', lastName);
    await box.put('preferredName', preferredName);
    if (dateOfBirth != null) {
      await box.put('dateOfBirth', dateOfBirth.toIso8601String());
    }
  }

  /// Get user's preferred name
  Future<String?> getUserName() async {
    final box = Hive.box<String>(_profileBox);
    return box.get('preferredName');
  }

  /// Get user's full name
  Future<String> getFullName() async {
    final box = Hive.box<String>(_profileBox);
    final first = box.get('firstName') ?? '';
    final middle = box.get('middleName') ?? '';
    final last = box.get('lastName') ?? '';
    if (middle.isNotEmpty) {
      return '$first $middle $last';
    }
    return '$first $last';
  }

  /// Get user's formal name
  Future<String> getFormalName() async {
    return await getFullName();
  }

  /// Check if onboarding is complete
  Future<bool> hasCompletedOnboarding() async {
    final box = Hive.box<String>(_profileBox);
    final preferredName = box.get('preferredName');
    debugPrint('ONBOARDING CHECK: preferredName in box = "$preferredName"');
    return preferredName != null && preferredName.isNotEmpty;
  }

  /// Regenerate old summaries with user's actual name
  Future<void> regenerateSummariesWithName(
    String userName,
    ContextCompressor compressor,
  ) async {
    for (final key in _openBox(_sessionsBox).keys.toList()) {
      final sessionStr = _openBox(_sessionsBox).get(key);
      if (sessionStr != null) {
        try {
          final session = InterviewSession.fromJson(
            jsonDecode(sessionStr) as Map<String, dynamic>);
          if (session.isComplete &&
              session.summary != null &&
              session.messages.length > 1 &&
              !session.summary!.contains(userName)) {
            debugPrint('Regenerating summary for ${session.id}');
            final newSummary = await compressor.summarizeSession(
              messages: session.messages);
            final updated = session.copyWith(summary: newSummary);
            await _openBox(_sessionsBox).put(key, jsonEncode(updated.toJson()));
          }
        } catch (e) {
          continue;
        }
      }
    }
  }

  /// Close all boxes
  Future<void> close() async {
    await _sessionBox.close();
    await _chapterBox.close();
    await _familyBoxInstance.close();
    await _photosBoxInstance.close();
    await _profileBoxInstance.close();
    await _narrativesBoxInstance.close();
    await _bookConfigBoxInstance.close();
  }
}
