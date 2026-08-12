# Sammy — Roadmap

Phased build order from the directive (§35). Each phase lists exit criteria.
Status of each phase lives in `STATE.md` and `PROJECT_STATE.json`.

## Phase 0 — Repository and Foundation  ✅ COMPLETE
Tauri 2 + Rust + React + TS + Vite; module boundaries; fmt/lint/test; canonical
docs; STATE tracking; basic 8-view navigation.
**Exit criteria:** dev build succeeds; production build succeeds; tests run via
documented commands; fmt + lint pass; no external repo dependency; STATE matches
reality. — **All verified 2026-08-05.**

## Phase 1 — Cryptographic Vault Spine  ✅ COMPLETE
Vault creation; master passphrase; recovery key; Argon2id derivation; random
data-encryption key; wrapped keys; SQLCipher DB; unlock/lock; inactivity +
session lock; multi-vault; encrypted settings; migrations; audit-event
foundation; initial backup/restore.
**Exit criteria:** no private access before unlock; wrong passwords fail safely;
recovery-key restore works; DB content not plaintext-readable; vaults isolated;
locking clears sensitive state; backup restores on a separate environment. — **All verified.**

## Phase 2 — Identity, Chat, and Providers  ✅ COMPLETE
Structured identity; conversation/message model; prompt assembly; mock +
KoboldCpp + Venice adapters; routing modes; cloud-crossing confirmation;
provider/model display; request audit events. Real KoboldCpp HTTP transport
verified live against localhost:5001. (Streaming + cancellation pending.)

## Phase 3 — Structured Knowledge  ✅ COMPLETE
Knowledge schema; record states; people/events/projects/facts/claims/
conclusions/preferences; What I Know; contradiction sets; corrections;
supersession; tombstones; knowledge audit; trust classifications.

## Phase 4 — Secure Source Ingestion  ✅ MOSTLY COMPLETE
File picker; encrypted source storage; checksums; txt/md/json/csv extractors;
FTS5 search; Sources UI. (PDF/DOCX/image/OCR extractors pending — interface ready.)

## Phase 5 — Retrieval and Citations  ✅ COMPLETE (with stubs)
FTS5; metadata/permission/domain filters; vector interface + stub adapter;
hybrid ranking; dedup; context budgeting; citation assembly; trust
classification. (Persistent vector index + real embedding adapter pending.)

## Phase 6 — Corpus Import and Export  ✅ COMPLETE
Versioned schemas; manifests; full export; import validation; stable IDs;
idempotent import; conflict detection; updates; supersession; tombstones;
CLI package validator. (Incremental export format pending.)

## Phase 7 — Auditable Memory  ✅ COMPLETE
Memory-candidate creation; review queue; approve/edit/reject/defer/temporary/
correct/supersede; conversation provenance; memory audit history.

## Phase 8 — Permission Architecture  ✅ COMPLETE
Tool registry; permission records; action preview; confirmation; allow-once;
session; narrow-scope; revocation; read-only/draft-only/mock tools; failure
handling; audit trail.

## Phase 9 — Product Validation  🚧 IN PROGRESS
Forbidden-behavior tests (§37); E2E workflow tests (§36); dependency license
report; real KoboldCpp connectivity verified live. Remaining: Windows
packaging/clean-machine install; performance at scale; streaming; user docs.

A roadmap, interface mockup, schema, or passing unit test alone does not satisfy
the definition of finished (`docs/RELEASE_ACCEPTANCE.md`).
