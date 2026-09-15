import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:hive_flutter/hive_flutter.dart';
import 'package:calli_archiviste/models/session.dart';
import 'package:calli_archiviste/models/artifact_metadata.dart';
import 'package:calli_archiviste/models/pkc_interview_evidence.dart';
import 'package:calli_archiviste/services/context_compressor.dart';

/// Artifact-intake scaffold storage (Calli-native).
///
/// Interview-world persistence lives in [SessionStorage] (donor-
/// extracted). Split 2026-09-15 during the engine extraction so that
/// both worlds compile side by side until Stage 3 reconciles them.
/// 
/// Storage is split into two boxes:
/// - knowledge_box: Contains PKC-bound knowledge (sessions, evidence, artifact metadata)
/// - ui_state_box: Contains ephemeral UI state (last screen, theme, scroll position)
class IntakeStorage {
  static const String _knowledgeBox = 'knowledge_box';
  static const String _uiStateBox = 'ui_state_box';

  late Box<String> _knowledgeBoxInstance;
  late Box<String> _uiStateBoxInstance;

  /// Initialize Hive and open boxes
  Future<void> initialize() async {
    _knowledgeBoxInstance = await Hive.openBox<String>(_knowledgeBox);
    _uiStateBoxInstance = await Hive.openBox<String>(_uiStateBox);
  }

  Box<String> get knowledgeBox => _knowledgeBoxInstance;
  Box<String> get uiStateBox => _uiStateBoxInstance;

  // ============ KNOWLEDGE BOX (PKC-bound) ============

  /// Save an intake session (knowledge)
  Future<void> saveSession(IntakeSession session) async {
    await _knowledgeBoxInstance.put(
      session.id,
      jsonEncode(session.toJson()),
    );
  }

  /// Get a session by ID
  Future<IntakeSession?> getSession(String sessionId) async {
    final json = _knowledgeBoxInstance.get(sessionId);
    if (json != null) {
      return IntakeSession.fromJson(
        jsonDecode(json) as Map<String, dynamic>,
      );
    }
    return null;
  }

  /// Get all sessions
  Future<List<IntakeSession>> getAllSessions() async {
    final sessions = <IntakeSession>[];
    for (final key in _knowledgeBoxInstance.keys) {
      final json = _knowledgeBoxInstance.get(key);
      if (json != null) {
        try {
          sessions.add(IntakeSession.fromJson(
            jsonDecode(json) as Map<String, dynamic>,
          ));
        } catch (e) {
          debugPrint('Failed to parse session $key: $e');
        }
      }
    }
    sessions.sort((a, b) => b.startedAt.compareTo(a.startedAt));
    return sessions;
  }

  /// Get sessions for a specific phase
  Future<List<IntakeSession>> getSessionsForPhase(IntakePhase phase) async {
    final allSessions = await getAllSessions();
    return allSessions.where((s) => s.phase == phase).toList();
  }

  /// Get completed sessions
  Future<List<IntakeSession>> getCompletedSessions() async {
    final allSessions = await getAllSessions();
    return allSessions.where((s) => s.isComplete).toList();
  }
/// Save artifact metadata (knowledge)
  Future<void> saveArtifactMetadata(ArtifactMetadata artifact) async {
    await _knowledgeBoxInstance.put(
      'artifact_${artifact.id}',
      jsonEncode(artifact.toJson()),
    );
  }

  /// Get artifact metadata by ID
  Future<ArtifactMetadata?> getArtifactMetadata(String artifactId) async {
    final json = _knowledgeBoxInstance.get('artifact_$artifactId');
    if (json != null) {
      return ArtifactMetadata.fromJson(
        jsonDecode(json) as Map<String, dynamic>,
      );
    }
    return null;
  }

  /// Get all artifact metadata
  Future<List<ArtifactMetadata>> getAllArtifactMetadata() async {
    final artifacts = <ArtifactMetadata>[];
    for (final key in _knowledgeBoxInstance.keys) {
      if (key.startsWith('artifact_')) {
        final json = _knowledgeBoxInstance.get(key);
        if (json != null) {
          try {
            artifacts.add(ArtifactMetadata.fromJson(
              jsonDecode(json) as Map<String, dynamic>,
            ));
          } catch (e) {
            debugPrint('Failed to parse artifact $key: $e');
          }
        }
      }
    }
    artifacts.sort((a, b) => b.addedAt.compareTo(a.addedAt));
    return artifacts;
  }

  /// Save PKC interview evidence (knowledge - pending submission)
  Future<void> savePkcEvidence(String sessionId, PKCInterviewEvidence evidence) async {
    await _knowledgeBoxInstance.put(
      'pkc_evidence_$sessionId',
      jsonEncode(evidence.toJson()),
    );
  }

  /// Get PKC interview evidence
  Future<PKCInterviewEvidence?> getPkcEvidence(String sessionId) async {
    final json = _knowledgeBoxInstance.get('pkc_evidence_$sessionId');
    if (json != null) {
      return PKCInterviewEvidence.fromJson(
        jsonDecode(json) as Map<String, dynamic>,
      );
    }
    return null;
  }

  /// Delete a session by ID
  Future<void> deleteSession(String sessionId) async {
    await _knowledgeBoxInstance.delete(sessionId);
    debugPrint('STORAGE DEBUG: Deleted session $sessionId');
  }

  // ============ UI STATE BOX (ephemeral, local only) ============

  /// Save UI state value
  Future<void> saveUiState(String key, String value) async {
    await _uiStateBoxInstance.put(key, value);
  }

  /// Get UI state value
  String? getUiState(String key) {
    return _uiStateBoxInstance.get(key);
  }

  /// Delete UI state value
  Future<void> deleteUiState(String key) async {
    await _uiStateBoxInstance.delete(key);
  }

  /// Clear all UI state (called on uninstall or session reset)
  Future<void> clearUiState() async {
    await _uiStateBoxInstance.clear();
  }

  /// Save user profile (knowledge - goes to knowledge box)
  Future<void> saveUserProfile({
    required String firstName,
    String? middleName,
    required String lastName,
    required String preferredName,
    DateTime? dateOfBirth,
  }) async {
    await _knowledgeBoxInstance.put('profile_firstName', firstName);
    if (middleName != null && middleName.isNotEmpty) {
      await _knowledgeBoxInstance.put('profile_middleName', middleName);
    }
    await _knowledgeBoxInstance.put('profile_lastName', lastName);
    await _knowledgeBoxInstance.put('profile_preferredName', preferredName);
    if (dateOfBirth != null) {
      await _knowledgeBoxInstance.put('profile_dateOfBirth', dateOfBirth.toIso8601String());
    }
  }

  /// Get user's preferred name
  Future<String?> getUserName() async {
    return _knowledgeBoxInstance.get('profile_preferredName');
  }

  /// Get user's full name
  Future<String> getFullName() async {
    final first = _knowledgeBoxInstance.get('profile_firstName') ?? '';
    final middle = _knowledgeBoxInstance.get('profile_middleName') ?? '';
    final last = _knowledgeBoxInstance.get('profile_lastName') ?? '';
    if (middle.isNotEmpty) {
      return '$first $middle $last';
    }
    return '$first $last';
  }

  /// Check if onboarding is complete
  Future<bool> hasCompletedOnboarding() async {
    final preferredName = _knowledgeBoxInstance.get('profile_preferredName');
    return preferredName != null && preferredName.isNotEmpty;
  }

  /// Regenerate old summaries with user's actual name
  Future<void> regenerateSummariesWithName(
    String userName,
    ContextCompressor compressor,
  ) async {
    for (final key in _knowledgeBoxInstance.keys.toList()) {
      if (!key.startsWith('artifact_') && !key.startsWith('pkc_evidence_') && !key.startsWith('profile_')) {
        final sessionStr = _knowledgeBoxInstance.get(key);
        if (sessionStr != null) {
          try {
            final session = IntakeSession.fromJson(
              jsonDecode(sessionStr) as Map<String, dynamic>);
            if (session.isComplete &&
                session.summary != null &&
                session.messages.length > 1 &&
                !session.summary!.contains(userName)) {
              debugPrint('Regenerating summary for ${session.id}');
              final newSummary = await compressor.summarizeSession(
                messages: session.messages);
              final updated = session.copyWith(summary: newSummary);
              await _knowledgeBoxInstance.put(key, jsonEncode(updated.toJson()));
            }
          } catch (e) {
            continue;
          }
        }
      }
    }
  }

  /// Close all boxes
  Future<void> close() async {
    await _knowledgeBoxInstance.close();
    await _uiStateBoxInstance.close();
  }
}