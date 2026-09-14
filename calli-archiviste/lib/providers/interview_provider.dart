import 'package:flutter/material.dart';
import 'package:calli_archiviste/services/session_storage.dart';
import 'package:calli_archiviste/services/interview_engine.dart';
import 'package:calli_archiviste/models/session.dart';
import 'package:calli_archiviste/models/message.dart';
import 'package:calli_archiviste/models/artifact_metadata.dart';
import 'package:calli_archiviste/models/intake_phase.dart';

/// Provider for interview UI state and session management
/// Delegates core logic to InterviewEngine
class InterviewProvider extends ChangeNotifier {
  final InterviewEngine _engine;
  final SessionStorage _storage;

  IntakeSession? _currentSession;
  bool _isGenerating = false;
  String? _error;
  List<ArtifactMetadata> _artifacts = [];

  InterviewProvider({
    required InterviewEngine engine,
    required SessionStorage storage,
  })  : _engine = engine,
        _storage = storage;

  IntakeSession? get currentSession => _currentSession;
  bool get isGenerating => _isGenerating;
  String? get error => _error;
  List<ArtifactMetadata> get artifacts => _artifacts;

  /// Start a new intake session
  Future<void> startNewSession(IntakePhase phase, {String? artifactId}) async {
    _currentSession = IntakeSession(
      phase: phase,
      artifactId: artifactId,
    );
    await _storage.saveSession(_currentSession!);
    notifyListeners();
  }

  /// Load an existing session
  Future<void> loadSession(String sessionId) async {
    _currentSession = await _storage.getSession(sessionId);
    notifyListeners();
  }

  /// Send a user message and get AI response
  Future<void> sendMessage(String content) async {
    if (_currentSession == null) return;

    _isGenerating = true;
    _error = null;
    notifyListeners();

    try {
      // Add user message
      final userMessage = Message(
        role: MessageRole.user,
        content: content,
      );
      _currentSession = _currentSession!.copyWith(
        messages: [..._currentSession!.messages, userMessage],
      );
      await _storage.saveSession(_currentSession!);

      // Generate AI response
      final response = await _engine.generateNextQuestion(
        session: _currentSession!,
      );

      final aiMessage = Message(
        role: MessageRole.interviewer,
        content: response,
      );
      _currentSession = _currentSession!.copyWith(
        messages: [..._currentSession!.messages, aiMessage],
      );
      await _storage.saveSession(_currentSession!);
    } catch (e) {
      _error = e.toString();
    } finally {
      _isGenerating = false;
      notifyListeners();
    }
  }

  /// End the current session
  Future<void> endSession() async {
    if (_currentSession == null) return;

    _currentSession = _currentSession!.copyWith(
      endedAt: DateTime.now(),
      isComplete: true,
    );
    await _storage.saveSession(_currentSession!);
    notifyListeners();
  }

  /// Save artifact metadata
  Future<void> saveArtifact(ArtifactMetadata artifact) async {
    await _storage.saveArtifactMetadata(artifact);
    _artifacts = await _storage.getAllArtifactMetadata();
    notifyListeners();
  }

  /// Load all artifacts
  Future<void> loadArtifacts() async {
    _artifacts = await _storage.getAllArtifactMetadata();
    notifyListeners();
  }

  /// Clear error
  void clearError() {
    _error = null;
    notifyListeners();
  }
}