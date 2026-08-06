# Sammy — Roadmap

Phased build order from the directive (§35). Each phase lists exit criteria.
Status of each phase lives in `STATE.md` and `PROJECT_STATE.json`.

## Phase 0 — Repository and Foundation  ✅ COMPLETE
Tauri 2 + Rust + React + TS + Vite; module boundaries; fmt/lint/test; canonical
docs; STATE tracking; basic 8-view navigation.
**Exit criteria:** dev build succeeds; production build succeeds; tests run via
documented commands; fmt + lint pass; no external repo dependency; STATE matches
reality. — **All verified 2026-08-05.**

## Phase 1 — Cryptographic Vault Spine  🚧 IN PROGRESS
Vault creation; master passphrase; recovery key; Argon2id derivation; random
data-encryption key; wrapped keys; SQLCipher DB; unlock/lock; inactivity +
session lock; multi-vault; encrypted settings; migrations; audit-event
foundation; initial backup/restore.
**Exit criteria:** no private access before unlock; wrong passwords fail safely;
recovery-key restore works; DB content not plaintext-readable; vaults isolated;
locking clears sensitive state; backup restores on a separate environment.

## Phase 2 — Identity, Chat, and Providers
Structured identity; conversation/message model; prompt assembly; mock +
KoboldCpp + Venice adapters; routing modes; cloud-crossing confirmation;
streaming + cancellation; provider/model display; request audit events.

## Phase 3 — Structured Knowledge
Knowledge schema; record states; people/events/projects/facts/claims/
conclusions/preferences; What I Know; contradiction sets; corrections;
supersession; tombstones; knowledge audit; trust classifications.

## Phase 4 — Secure Source Ingestion
File picker; folder grants; encrypted source storage; checksums; txt/md/pdf/
docx/json/csv extractors; image metadata; OCR adapter; resumable import jobs;
cancellation; plaintext-temp cleanup.

## Phase 5 — Retrieval and Citations
FTS5; metadata/permission/domain filters; vector interface + local adapter;
hybrid ranking; dedup; context budgeting; citation assembly; trust
classification; source browser; reindexing; deleted-source removal.

## Phase 6 — Corpus Import and Export
Versioned schemas; manifests; full/incremental export; import validation; stable
IDs; idempotent import; conflict detection; updates; supersession; tombstones;
import history; index rebuild; CLI package validator; fixtures.

## Phase 7 — Auditable Memory
Memory-candidate creation; review queue; approve/edit/reject/defer/temporary/
correct/supersede; conversation provenance; memory audit history.

## Phase 8 — Permission Architecture
Tool registry; permission records; action preview; confirmation; allow-once;
session; narrow-scope; revocation; read-only/draft-only/mock tools; failure
handling; audit trail.

## Phase 9 — Product Validation
Windows packaging; clean-machine install; upgrade migration; full backup
restore; recovery-key restore; corrupted-backup tests; wrong-passphrase;
retrieval quality; provider-switch personality; security regression;
threat-model review; performance at scale; failure/interruption recovery; user
+ developer docs; dependency license report.

A roadmap, interface mockup, schema, or passing unit test alone does not satisfy
the definition of finished (`docs/RELEASE_ACCEPTANCE.md`).
