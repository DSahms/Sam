import 'dart:io';
import 'package:image_picker/image_picker.dart';
import 'package:path/path.dart' as path;
import 'package:uuid/uuid.dart';
import 'package:calli_archiviste/models/artifact_metadata.dart';
import 'package:calli_archiviste/models/intake_phase.dart';

/// Abstract interface for Media Archive
/// This allows Calli to store artifacts without knowing the implementation details
abstract class MediaArchive {
  /// Save an artifact and return its stable ID
  Future<String> saveArtifact({
    required File file,
    required String mimeType,
    Map<String, dynamic>? metadata,
  });

  /// Get artifact file by ID
  Future<File?> getArtifact(String artifactId);

  /// Get artifact metadata by ID
  Future<ArtifactMetadata?> getArtifactMetadata(String artifactId);

  /// Delete artifact
  Future<void> deleteArtifact(String artifactId);

  /// Compute SHA256 hash of a file
  Future<String> computeHash(File file);
}

/// Stub implementation for testing
class StubMediaArchive implements MediaArchive {
  final Map<String, File> _artifacts = {};
  final Map<String, ArtifactMetadata> _metadata = {};

  @override
  Future<String> saveArtifact({
    required File file,
    required String mimeType,
    Map<String, dynamic>? metadata,
  }) async {
    final id = const Uuid().v4();
    _artifacts[id] = file;
    _metadata[id] = ArtifactMetadata(
      id: id,
      filePath: file.path,
      phase: IntakePhase.artifactIdentification,
      addedAt: DateTime.now(),
      mimeType: mimeType,
      fileSize: await file.length(),
      sha256Hash: await computeHash(file),
    );
    return id;
  }

  @override
  Future<File?> getArtifact(String artifactId) async {
    return _artifacts[artifactId];
  }

  @override
  Future<ArtifactMetadata?> getArtifactMetadata(String artifactId) async {
    return _metadata[artifactId];
  }

  @override
  Future<void> deleteArtifact(String artifactId) async {
    _artifacts.remove(artifactId);
    _metadata.remove(artifactId);
  }

  @override
  Future<String> computeHash(File file) async {
    // Simple stub - in real implementation would use crypto
    return 'sha256_${file.path.hashCode}';
  }
}

/// Service for managing artifact capture
/// Delegates storage to MediaArchive
class MediaService {
  final MediaArchive _mediaArchive;
  final ImagePicker _picker = ImagePicker();

  MediaService(this._mediaArchive);

  /// Pick a photo from gallery
  /// Returns the artifact ID after saving to Media Archive
  Future<String?> pickPhoto() async {
    final image = await _picker.pickImage(
      source: ImageSource.gallery,
      maxWidth: 1920,
      maxHeight: 1920,
      imageQuality: 85,
    );
    if (image == null) return null;

    final file = File(image.path);
    return _mediaArchive.saveArtifact(
      file: file,
      mimeType: 'image/${path.extension(image.path).replaceFirst('.', '')}',
    );
  }

  /// Take a photo with camera
  /// Returns the artifact ID after saving to Media Archive
  Future<String?> takePhoto() async {
    final image = await _picker.pickImage(
      source: ImageSource.camera,
      maxWidth: 1920,
      maxHeight: 1920,
      imageQuality: 85,
    );
    if (image == null) return null;

    final file = File(image.path);
    return _mediaArchive.saveArtifact(
      file: file,
      mimeType: 'image/${path.extension(image.path).replaceFirst('.', '')}',
    );
  }

  /// Pick a document/file
  Future<String?> pickDocument() async {
    // TODO: Implement document picker
    return null;
  }

  /// Record audio
  Future<String?> recordAudio() async {
    // TODO: Implement audio recording
    return null;
  }
}