enum PKCInterviewOperation {
  metadataDiscovery('metadata_discovery'),
  protectedRetrieval('protected_retrieval');

  const PKCInterviewOperation(this.wireValue);
  final String wireValue;
}

class PKCInterviewRequest {
  const PKCInterviewRequest({
    required this.operation,
    required this.question,
    required this.chapterContext,
    this.requestId = 'CALLI-FLUTTER-PRODUCTION',
    this.consumerApplication = 'calli_archiviste',
    this.recipientClass = 'owner_dave',
    this.realm = 'private_autobiographical_interview',
    this.purpose = 'interview_dave',
    this.disclosureMode = 'private_local',
    this.requestedContentLevel = 'metadata',
    this.consentState = 'missing',
    this.sourceId,
    this.sourceHandles = const [],
  });

  static const bridgeVersion = '1.0.0';

  final PKCInterviewOperation operation;
  final String question;
  final String chapterContext;
  final String requestId;
  final String consumerApplication;
  final String recipientClass;
  final String realm;
  final String purpose;
  final String disclosureMode;
  final String requestedContentLevel;
  final String consentState;
  final String? sourceId;
  final List<String> sourceHandles;

  Map<String, Object?> toJson() => {
    'bridge_version': bridgeVersion,
    'request_id': requestId,
    'operation': operation.wireValue,
    'question': question,
    'chapter_context': chapterContext,
    'consumer_application': consumerApplication,
    'recipient_class': recipientClass,
    'realm': realm,
    'purpose': purpose,
    'disclosure_mode': disclosureMode,
    'requested_content_level': requestedContentLevel,
    'consent_state': consentState,
    if (sourceId != null) 'source_id': sourceId,
    if (sourceHandles.isNotEmpty) 'source_handles': sourceHandles,
  };
}

class PKCSourceRelationship {
  const PKCSourceRelationship({required this.type, required this.target});

  final String type;
  final String target;

  factory PKCSourceRelationship.fromJson(Map<String, dynamic> json) =>
      PKCSourceRelationship(
        type: json['type'] as String? ?? '',
        target: json['target'] as String? ?? '',
      );
}

class PKCEvidenceSource {
  const PKCEvidenceSource({
    required this.id,
    required this.title,
    required this.sourceKind,
    required this.bodyIncluded,
    required this.relationships,
  });

  final String id;
  final String? title;
  final String? sourceKind;
  final bool bodyIncluded;
  final List<PKCSourceRelationship> relationships;

  factory PKCEvidenceSource.fromJson(Map<String, dynamic> json) {
    final relationships = json['relationships'];
    return PKCEvidenceSource(
      id: json['id'] as String? ?? '',
      title: json['title'] as String?,
      sourceKind: json['source_kind'] as String?,
      bodyIncluded: json['body_included'] as bool? ?? false,
      relationships: relationships is List
          ? relationships
                .whereType<Map>()
                .map(
                  (item) => PKCSourceRelationship.fromJson(
                    Map<String, dynamic>.from(item),
                  ),
                )
                .toList()
          : const [],
    );
  }
}

class PKCInterviewEvidence {
  const PKCInterviewEvidence({
    required this.ok,
    required this.available,
    required this.naturalAnswer,
    required this.conversationalContext,
    required this.supportStatus,
    required this.sourceHandles,
    required this.sources,
    required this.withheld,
    required this.safeReason,
    required this.authorized,
    required this.canonicalHashVerified,
    required this.protectedContentMaterialized,
  });

  final bool ok;
  final bool available;
  final String naturalAnswer;
  final String conversationalContext;
  final String supportStatus;
  final List<String> sourceHandles;
  final List<PKCEvidenceSource> sources;
  final bool withheld;
  final String? safeReason;
  final bool authorized;
  final bool canonicalHashVerified;
  final bool protectedContentMaterialized;

  String get conversationalPayload => [
    naturalAnswer,
    conversationalContext,
  ].where((value) => value.isNotEmpty).join('\n');

  factory PKCInterviewEvidence.unavailable(String safeReason) =>
      PKCInterviewEvidence(
        ok: false,
        available: false,
        naturalAnswer: '',
        conversationalContext: '',
        supportStatus: 'Unknown',
        sourceHandles: const [],
        sources: const [],
        withheld: true,
        safeReason: safeReason,
        authorized: false,
        canonicalHashVerified: false,
        protectedContentMaterialized: false,
      );

  factory PKCInterviewEvidence.fromBridgeJson(Map<String, dynamic> json) {
    if (json['bridge_version'] != PKCInterviewRequest.bridgeVersion) {
      throw const FormatException('Unsupported PKC bridge response');
    }
    final rawPayload = json['payload'];
    if (rawPayload is! Map) {
      throw const FormatException('Malformed PKC bridge payload');
    }
    final payload = Map<String, dynamic>.from(rawPayload);
    final handles = payload['source_handles'];
    final sources = payload['sources'];
    return PKCInterviewEvidence(
      ok: json['ok'] as bool? ?? false,
      available: json['available'] as bool? ?? false,
      naturalAnswer: payload['natural_answer'] as String? ?? '',
      conversationalContext: payload['conversational_context'] as String? ?? '',
      supportStatus: payload['support_status'] as String? ?? 'Unknown',
      sourceHandles: handles is List
          ? handles.whereType<String>().toList()
          : const [],
      sources: sources is List
          ? sources
                .whereType<Map>()
                .map(
                  (item) => PKCEvidenceSource.fromJson(
                    Map<String, dynamic>.from(item),
                  ),
                )
                .toList()
          : const [],
      withheld: payload['withheld'] as bool? ?? true,
      safeReason: payload['safe_reason'] as String?,
      authorized: payload['authorized'] as bool? ?? false,
      canonicalHashVerified:
          payload['canonical_hash_verified'] as bool? ?? false,
      protectedContentMaterialized:
          payload['protected_content_materialized'] as bool? ?? false,
    );
  }
}

class PKCBackedFollowUpResult {
  const PKCBackedFollowUpResult({
    required this.followUp,
    required this.evidence,
    required this.usedPkcContext,
    required this.localModelAttempted,
    required this.usedFallback,
  });

  final String followUp;
  final PKCInterviewEvidence evidence;
  final bool usedPkcContext;
  final bool localModelAttempted;
  final bool usedFallback;
}
