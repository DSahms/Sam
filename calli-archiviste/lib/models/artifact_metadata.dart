import 'package:calli_archiviste/models/intake_phase.dart';

/// Metadata for a captured artifact (photo, document, audio, etc.)
class ArtifactMetadata {
  final String id;              // UUID
  final String filePath;        // Local file path to artifact
  final IntakePhase phase;      // Which intake phase this artifact relates to
  final DateTime addedAt;       // When the artifact was added
  String? caption;              // User-provided caption
  String? aiDescription;       // AI-generated description (future use)
  List<String> peopleInArtifact; // Names of people identified
  String? location;             // Where the artifact was taken/created
  String? year;                 // Approximate year of artifact
  String? notes;                // Free-form user notes
  String? mimeType;            // File type (image/jpeg, application/pdf, etc.)
  int? fileSize;                // Size in bytes
  String? sha256Hash;          // Provenance hash

  ArtifactMetadata({
    required this.id,
    required this.filePath,
    required this.phase,
    required this.addedAt,
    this.caption,
    this.aiDescription,
    this.peopleInArtifact = const [],
    this.location,
    this.year,
    this.notes,
    this.mimeType,
    this.fileSize,
    this.sha256Hash,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'filePath': filePath,
        'phase': phase.toString(),
        'addedAt': addedAt.toIso8601String(),
        'caption': caption,
        'aiDescription': aiDescription,
        'peopleInArtifact': peopleInArtifact,
        'location': location,
        'year': year,
        'notes': notes,
        'mimeType': mimeType,
        'fileSize': fileSize,
        'sha256Hash': sha256Hash,
      };

  factory ArtifactMetadata.fromJson(Map<String, dynamic> json) =>
      ArtifactMetadata(
        id: json['id'] as String,
        filePath: json['filePath'] as String,
        phase: IntakePhase.values.firstWhere(
          (p) => p.toString() == json['phase'],
          orElse: () => IntakePhase.artifactIdentification,
        ),
        addedAt: DateTime.parse(json['addedAt'] as String),
        caption: json['caption'] as String?,
        aiDescription: json['aiDescription'] as String?,
        peopleInArtifact: (json['peopleInArtifact'] as List<dynamic>?)
                ?.cast<String>() ??
            [],
        location: json['location'] as String?,
        year: json['year'] as String?,
        notes: json['notes'] as String?,
        mimeType: json['mimeType'] as String?,
        fileSize: json['fileSize'] as int?,
        sha256Hash: json['sha256Hash'] as String?,
      );

  ArtifactMetadata copyWith({
    String? id,
    String? filePath,
    IntakePhase? phase,
    DateTime? addedAt,
    String? caption,
    String? aiDescription,
    List<String>? peopleInArtifact,
    String? location,
    String? year,
    String? notes,
    String? mimeType,
    int? fileSize,
    String? sha256Hash,
  }) {
    return ArtifactMetadata(
      id: id ?? this.id,
      filePath: filePath ?? this.filePath,
      phase: phase ?? this.phase,
      addedAt: addedAt ?? this.addedAt,
      caption: caption ?? this.caption,
      aiDescription: aiDescription ?? this.aiDescription,
      peopleInArtifact: peopleInArtifact ?? this.peopleInArtifact,
      location: location ?? this.location,
      year: year ?? this.year,
      notes: notes ?? this.notes,
      mimeType: mimeType ?? this.mimeType,
      fileSize: fileSize ?? this.fileSize,
      sha256Hash: sha256Hash ?? this.sha256Hash,
    );
  }
}