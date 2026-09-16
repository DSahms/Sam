/// Record candidates and the gate policy — Stage 5-b-1.
///
/// This file is the Dart projection of the record superset schema defined in
/// docs/architecture/stage-5b-intake-registry.md (§3, §5) and the mint table
/// derived from PKC_LAYOUT_SPEC §5. It is deliberately pure Dart: no I/O, no
/// model calls, no Flutter imports. The engine owns state and decisions;
/// the model owns wording; Dave owns approval.
///
/// Law of the mint table (Stage 5-b design §2, invariant I-4):
/// the engine can only ever MINT candidates. Nothing this file produces is
/// approved, and nothing ships until Dave stamps the gate (registry layer,
/// Stage 5-b-2). Confidence values are the standing mint-table rates:
///
/// | record_type      | confidence | condition                          |
/// |------------------|------------|------------------------------------|
/// | event            | 1.0        | verbatim surface answer exists     |
/// | judgment         | 1.0–0.8    | chain reached contrast (+meaning)  |
/// | claim            | 0.6        | >=2 independent sources (registry) |
/// | memory_candidate | 0.3        | unresolved chain (skips below L5)  |
///
/// Claims are intentionally NOT mintable by the single-session engine: a
/// claim requires two independent sources, which only the registry layer
/// (Stage 5-b-2) can see across sessions. The 0.6 constant is pinned here
/// so the mint table lives in exactly one place.
library;

/// Gate actions Dave can perform on a record. Mirrors the registry
/// gate_log vocabulary (Stage 5-b design §6) one-to-one.
enum GateAction { approve, reject, reopen, close }

/// Record lifecycle status. The engine can only ever produce [candidate];
/// every other state is reachable only through the gate, by Dave.
enum RecordStatus { candidate, approved, rejected, reopened, closed }

/// PKC record types. Wire values match registry sqlite rows (§5).
enum RecordType {
  event('event'),
  judgment('judgment'),
  claim('claim'),
  memoryCandidate('memory_candidate');

  const RecordType(this.wireValue);
  final String wireValue;
}

/// Sensitivity tiers from PKC_LAYOUT_SPEC §6. S0 (publishable) and S1
/// (personal default) both map to `normal` on the wire; S2 is `sensitive`
/// (argon2-protected) and S3 is `local_only` (never leaves the machine).
enum PkcSensitivity {
  normal('normal'),
  sensitive('sensitive'),
  localOnly('local_only');

  const PkcSensitivity(this.wireValue);
  final String wireValue;
}

/// S-tier labels (PKC_LAYOUT_SPEC §6). Kept distinct from [PkcSensitivity]
/// because the tier is Dave's mental model and the sensitivity is the
/// storage behavior; the mapping (s0/s1 -> normal, s2 -> sensitive,
/// s3 -> localOnly) is applied at persistence time by the registry.
enum PkcTier { s0, s1, s2, s3 }

/// One candidate record produced by [mint] on the probe chain engine.
///
/// [canonicalText] is verbatim (invariant I-3): the engine never trims,
/// normalizes, or "improves" it. Sanitization happens only at the export
/// compiler (06_exports), never on canonical text — STORE TRUTH /
/// SANITIZE EXPORTS.
class RecordCandidate {
  const RecordCandidate({
    required this.recordId,
    required this.recordType,
    required this.canonicalText,
    required this.confidence,
    this.status = RecordStatus.candidate,
    this.sensitivity = PkcSensitivity.normal,
    this.pkcTier = PkcTier.s1,
    this.probeChain = const [],
    this.sources = const [],
    this.createdAt,
  });

  /// Standing mint-table rates. Single source of truth; do not inline.
  static const double eventConfidence = 1.0;
  static const double judgmentConfidenceFull = 1.0;
  static const double judgmentConfidenceContrast = 0.8;
  static const double claimConfidence = 0.6;
  static const double memoryCandidateConfidence = 0.3;

  final String recordId;
  final RecordType recordType;
  final String canonicalText;
  final double confidence;
  final RecordStatus status;
  final PkcSensitivity sensitivity;
  final PkcTier pkcTier;

  /// Probe level names this record's chain actually walked, in walk order
  /// (e.g. ['surface', 'sensory', 'source']). Skipped levels are recorded
  /// as 'surface.skip' style entries so the audit trail shows the poison.
  final List<String> probeChain;

  /// Source locators (e.g. '@chat:2026-09-16#msg-16'). A single-session
  /// chain contributes exactly one locator; claims need two, which is why
  /// the engine never mints them.
  final List<String> sources;

  final DateTime? createdAt;

  bool get isCandidateOnly =>
      status == RecordStatus.candidate && recordType != RecordType.claim;

  Map<String, Object?> toJson() => {
    'schema_version': schemaVersion,
    'record_id': recordId,
    'record_type': recordType.wireValue,
    'canonical_text': canonicalText,
    'confidence': confidence,
    'status': status.name,
    'sensitivity': sensitivity.wireValue,
    'pkc_tier': pkcTier.name,
    'probe_chain': probeChain,
    'sources': sources,
    if (createdAt != null) 'created_at': createdAt!.toIso8601String(),
  };

  /// Storage schema version for the candidate projection. Bump only when a
  /// field changes meaning, not when fields are added (lenient fromJson).
  static const int schemaVersion = 1;

  factory RecordCandidate.fromJson(Map<String, dynamic> json) {
    final chain = json['probe_chain'];
    final srcs = json['sources'];
    final created = json['created_at'];
    return RecordCandidate(
      recordId: json['record_id'] as String? ?? '',
      recordType: RecordType.values.firstWhere(
        (t) => t.wireValue == (json['record_type'] as String? ?? ''),
        orElse: () => RecordType.memoryCandidate,
      ),
      canonicalText: json['canonical_text'] as String? ?? '',
      confidence:
          (json['confidence'] as num?)?.toDouble() ??
          RecordCandidate.memoryCandidateConfidence,
      status: RecordStatus.values.firstWhere(
        (s) => s.name == (json['status'] as String? ?? ''),
        orElse: () => RecordStatus.candidate,
      ),
      sensitivity: PkcSensitivity.values.firstWhere(
        (s) => s.wireValue == (json['sensitivity'] as String? ?? ''),
        orElse: () => PkcSensitivity.normal,
      ),
      pkcTier: PkcTier.values.firstWhere(
        (t) => t.name == (json['pkc_tier'] as String? ?? ''),
        orElse: () => PkcTier.s1,
      ),
      probeChain: chain is List ? chain.whereType<String>().toList() : const [],
      sources: srcs is List ? srcs.whereType<String>().toList() : const [],
      createdAt:
          created is String ? DateTime.tryParse(created)?.toUtc() : null,
    );
  }
}

/// The gate. Law 3 of the corpus: closures and approvals are stamped BY
/// DAVE, in person, and everything else is denied (Stage 5-b design §6).
///
/// The engine calls [evaluate] defensively; the registry layer (5-b-2)
/// re-checks it before writing gate_log. Two independent rejections of the
/// same non-Dave actor is the design working, not an accident.
class GatePolicy {
  const GatePolicy._();

  /// The only actor the gate ever accepts. Everything else — including the
  /// engine itself, the model, automation, and scripts — is denied.
  static const String approvedActor = 'dave';

  static GateDecision evaluate({
    required String actor,
    required GateAction action,
    required RecordStatus currentStatus,
    RecordType recordType = RecordType.judgment,
  }) {
    if (actor != approvedActor) {
      return GateDecision.denied(
        'gate actor must be "$approvedActor"; got "$actor" '
        '(I-4: the engine cannot approve, and neither can anything else '
        'that is not Dave)',
      );
    }
    switch (action) {
      case GateAction.approve:
      case GateAction.reject:
        if (currentStatus != RecordStatus.candidate &&
            currentStatus != RecordStatus.reopened) {
          return GateDecision.denied(
            'cannot ${action.name} a record in status ${currentStatus.name}',
          );
        }
        return GateDecision.granted(action);
      case GateAction.reopen:
        if (currentStatus != RecordStatus.approved &&
            currentStatus != RecordStatus.closed) {
          return GateDecision.denied(
            'cannot reopen a record in status ${currentStatus.name}',
          );
        }
        return GateDecision.granted(action);
      case GateAction.close:
        // I-2 closure rule: close requires an approved record. The Dave-only
        // manual override for below-L4 chains is a gate_log event written at
        // the registry layer (5-b-2) — the policy here stays strict so the
        // code path itself cannot close anything Dave has not approved.
        if (currentStatus != RecordStatus.approved) {
          return GateDecision.denied(
            'cannot close a record in status ${currentStatus.name} '
            '(I-2: only Dave-approved records close)',
          );
        }
        return GateDecision.granted(action);
    }
  }
}

/// Result of a gate evaluation. Granted decisions carry the action so the
/// registry can stamp gate_log with exactly what was authorized.
class GateDecision {
  const GateDecision.granted(this.action)
    : allowed = true,
      reason = null;

  const GateDecision.denied(this.reason)
    : allowed = false,
      action = null;

  final bool allowed;
  final GateAction? action;
  final String? reason;
}
