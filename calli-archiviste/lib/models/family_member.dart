class FamilyMember {
  final String id;              // UUID
  String name;
  String? nickname;
  String relationship;          // 'mother', 'father', 'paternal grandmother', etc.
  DateTime? birthDate;
  DateTime? deathDate;
  String? birthPlace;
  String? physicalDescription;  // Hair color, build, features
  List<String> traits;          // 'funny', 'strict', 'quiet', 'generous'
  String? occupation;
  String? notes;                // Free-form user notes
  String? parentId;             // Links to parent node in tree
  List<String> childIds;        // Links to children nodes

  FamilyMember({
    required this.id,
    required this.name,
    required this.relationship,
    this.nickname,
    this.birthDate,
    this.deathDate,
    this.birthPlace,
    this.physicalDescription,
    this.traits = const [],
    this.occupation,
    this.notes,
    this.parentId,
    this.childIds = const [],
  });

  /// Format for interview context - produces a natural
  /// language description for the AI interviewer
  String toContextString() {
    final parts = <String>[];
    parts.add('$name ($relationship)');
    if (nickname != null) parts.add('also called "$nickname"');
    if (occupation != null) parts.add('worked as $occupation');
    if (birthPlace != null) parts.add('from $birthPlace');
    if (physicalDescription != null) parts.add(physicalDescription!);
    if (traits.isNotEmpty) parts.add('described as ${traits.join(", ")}');
    if (deathDate != null) parts.add('deceased');
    if (notes != null) parts.add('Notes: $notes');
    return parts.join('. ') + '.';
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'nickname': nickname,
    'relationship': relationship,
    'birthDate': birthDate?.toIso8601String(),
    'deathDate': deathDate?.toIso8601String(),
    'birthPlace': birthPlace,
    'physicalDescription': physicalDescription,
    'traits': traits,
    'occupation': occupation,
    'notes': notes,
    'parentId': parentId,
    'childIds': childIds,
  };

  factory FamilyMember.fromJson(Map<String, dynamic> json) =>
    FamilyMember(
      id: json['id'] as String,
      name: json['name'] as String,
      relationship: json['relationship'] as String,
      nickname: json['nickname'] as String?,
      birthDate: json['birthDate'] != null
        ? DateTime.parse(json['birthDate'] as String) : null,
      deathDate: json['deathDate'] != null
        ? DateTime.parse(json['deathDate'] as String) : null,
      birthPlace: json['birthPlace'] as String?,
      physicalDescription: json['physicalDescription'] as String?,
      traits: (json['traits'] as List<dynamic>?)
        ?.cast<String>() ?? [],
      occupation: json['occupation'] as String?,
      notes: json['notes'] as String?,
      parentId: json['parentId'] as String?,
      childIds: (json['childIds'] as List<dynamic>?)
        ?.cast<String>() ?? [],
    );
}
