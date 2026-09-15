import 'package:flutter/material.dart';
import 'package:calli_archiviste/config/constants.dart';
import 'package:calli_archiviste/models/artifact_metadata.dart';
import 'package:calli_archiviste/models/family_member.dart';
import 'package:calli_archiviste/models/interview_session.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/models/photo_metadata.dart';
import 'package:calli_archiviste/services/intake_storage.dart';
import 'package:calli_archiviste/services/interview_engine.dart';
import 'package:calli_archiviste/services/session_storage.dart';

/// Provider for the intake interview UI.
///
/// Adapted 2026-09-15 to drive the authoritative donor InterviewEngine
/// (extraction per docs/migration/calli-migration-plan.md and the master
/// architecture doc section 17: exactly one interview engine). The engine,
/// LLM backend and storages are injected by the host application; this
/// provider only orchestrates UI state.
class InterviewProvider extends ChangeNotifier {
  final InterviewEngine _engine;
  final SessionStorage _storage;
  final IntakeStorage _intakeStorage;

  InterviewSession? _currentSession;
  bool _isGenerating = false;
  bool _engineInitialized = false;
  String? _error;
  List<ArtifactMetadata> _artifacts = [];
  List<FamilyMember> _familyMembers = [];
  List<PhotoMetadata> _photos = [];

  InterviewProvider({
    required InterviewEngine engine,
    required SessionStorage storage,
    required IntakeStorage intakeStorage,
  })  : _engine = engine,
        _storage = storage,
        _intakeStorage = intakeStorage;

  InterviewSession? get currentSession => _currentSession;
  bool get isGenerating => _isGenerating;
  String? get error => _error;
  List<ArtifactMetadata> get artifacts => _artifacts;
  List<FamilyMember> get familyMembers => _familyMembers;
  List<PhotoMetadata> get photos => _photos;

  /// Load donor prompt assets once. Safe to call repeatedly.
  Future<void> ensureEngineInitialized() async {
    if (_engineInitialized) return;
    await _engine.initialize();
    _engineInitialized = true;
  }

  /// Start a new interview session for [chapter].
  Future<void> startNewSession(LifeChapter chapter) async {
    await ensureEngineInitialized();
    _familyMembers = await _storage.getAllFamilyMembers();
    _photos = await _storage.getPhotosForChapter(chapter);
    _currentSession = InterviewSession(chapter: chapter);
    try {
      final opening = await _engine.generateOpeningQuestion(chapter, null);
      _currentSession = _currentSession!.copyWith(
        messages: [
          Message(role: MessageRole.interviewer, content: opening),
        ],
      );
    } catch (e) {
      // Opening question is best-effort; the interview can still proceed.
    }
    await _storage.saveSession(_currentSession!);
    notifyListeners();
  }

  /// Load an existing interview session.
  Future<void> loadSession(String sessionId) async {
    await ensureEngineInitialized();
    _currentSession = await _storage.getSession(sessionId);
    notifyListeners();
  }

  /// Send a user answer and generate the next interview question.
  Future<void> sendMessage(String content) async {
    final session = _currentSession;
    if (session == null || _isGenerating) return;

    _isGenerating = true;
    _error = null;
    notifyListeners();

    try {
      var updated = session.copyWith(
        messages: [
          ...session.messages,
          Message(role: MessageRole.user, content: content),
        ],
      );
      await _storage.saveSession(updated);

      final question = await _engine.generateNextQuestion(
        updated,
        updated.messages,
        familyMembers: _familyMembers,
        photos: _photos,
      );

      updated = updated.copyWith(
        messages: [
          ...updated.messages,
          Message(role: MessageRole.interviewer, content: question),
        ],
      );
      await _storage.saveSession(updated);
      _currentSession = updated;
    } catch (e) {
      _error = e.toString();
    } finally {
      _isGenerating = false;
      notifyListeners();
    }
  }

  /// End the current session.
  Future<void> endSession() async {
    final session = _currentSession;
    if (session == null) return;
    _currentSession = session.copyWith(
      endedAt: DateTime.now(),
      isComplete: true,
    );
    await _storage.saveSession(_currentSession!);
    notifyListeners();
  }

  // ==== Artifact intake (Calli scaffold, persisted via IntakeStorage) ====

  /// Save artifact metadata.
  Future<void> saveArtifact(ArtifactMetadata artifact) async {
    await _intakeStorage.saveArtifactMetadata(artifact);
    _artifacts = await _intakeStorage.getAllArtifactMetadata();
    notifyListeners();
  }

  /// Load all artifacts.
  Future<void> loadArtifacts() async {
    _artifacts = await _intakeStorage.getAllArtifactMetadata();
    notifyListeners();
  }

  /// Clear error.
  void clearError() {
    _error = null;
    notifyListeners();
  }
}
