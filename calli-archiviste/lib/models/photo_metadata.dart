import 'package:calli_archiviste/config/constants.dart';

class PhotoMetadata {
  final String id;              // UUID
  final String filePath;        // Local file path to image
  final LifeChapter chapter;    // Which chapter this photo belongs to
  final DateTime addedAt;       // When the photo was added
  String? caption;              // User-provided caption
  String? aiDescription;        // AI-generated description (future use)
  List<String> peopleInPhoto;   // Names of people identified
  String? location;             // Where the photo was taken
  String? year;                 // Approximate year of photo
  String? notes;                // Free-form user notes

  PhotoMetadata({
    required this.id,
    required this.filePath,
    required this.chapter,
    required this.addedAt,
    this.caption,
    this.aiDescription,
    this.peopleInPhoto = const [],
    this.location,
    this.year,
    this.notes,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'filePath': filePath,
    'chapter': chapter.name,
    'addedAt': addedAt.toIso8601String(),
    'caption': caption,
    'aiDescription': aiDescription,
    'peopleInPhoto': peopleInPhoto,
    'location': location,
    'year': year,
    'notes': notes,
  };

  factory PhotoMetadata.fromJson(Map<String, dynamic> json) =>
    PhotoMetadata(
      id: json['id'] as String,
      filePath: json['filePath'] as String,
      chapter: LifeChapter.values.firstWhere(
        (c) => c.name == json['chapter']),
      addedAt: DateTime.parse(json['addedAt'] as String),
      caption: json['caption'] as String?,
      aiDescription: json['aiDescription'] as String?,
      peopleInPhoto: (json['peopleInPhoto'] as List<dynamic>?)
        ?.cast<String>() ?? [],
      location: json['location'] as String?,
      year: json['year'] as String?,
      notes: json['notes'] as String?,
    );
}
