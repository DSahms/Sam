# Sammy — Requirements

Requirements derived from the project directive. Each is numbered `R<phase>.<n>`
and traced to the directive section. Acceptance is in `docs/RELEASE_ACCEPTANCE.md`.

## R0 — Foundation (Phase 0)

- **R0.1** Tauri 2 + Rust + React + TypeScript + Vite application builds and runs. (§4, §41)
- **R0.2** `cargo fmt`, `cargo clippy -D warnings`, ESLint, Prettier, TS strict configured and passing. (§4)
- **R0.3** Rust unit + integration and TS unit/component test frameworks configured. (§4)
- **R0.4** Repeatable dev and production build commands; CI-ready scripts. (§4)
- **R0.5** Module boundaries for every subsystem in §7 exist as Rust modules. (§7)
- **R0.6** Canonical project documents and `STATE.md` / `PROJECT_STATE.json` exist and match reality. (§34)
- **R0.7** Basic application navigation across the eight primary views. (§33)
- **R0.8** No external repository dependency; no sibling-repo inspection. (§5)

## R1 — Cryptographic vault spine (Phase 1)

- **R1.1** Multiple fully-independent vaults, each with its own random DEK, SQLCipher DB, source store, conversations, knowledge, indexes, identity, memory candidates, provider routes, permissions, audit, backup. (§9)
- **R1.2** Argon2id KDF from master passphrase → key-encryption key → wraps DEK. (§10)
- **R1.3** High-entropy recovery key wraps DEK separately; shown once; confirmable; never stored. (§11)
- **R1.4** Vault unlock/lock; inactivity lock (default 15 min) with options; lock on exit, session lock, manual. (§10)
- **R1.5** Wrong-passphrase safe failure; no plaintext passphrase/key/hint stored. (§10, §11)
- **R1.6** DB content not readable as plaintext; vaults cannot access one another; no cross-vault retrieval. (§8, §9)
- **R1.7** Encrypted settings; provider credentials encrypted at rest. (§13)
- **R1.8** Versioned DB migrations with rollback protection. (§4, §37)
- **R1.9** Append-only audit-event foundation. (§7, §33)
- **R1.10** Initial full encrypted backup + restore, restorable on a separate environment via passphrase or recovery key. (§30, §31)

## R2 — Identity, chat, providers (Phase 2)

Structured provider-independent identity (§12); conversation/message model &
prompt assembly; mock + KoboldCpp + Venice adapters behind a stable interface;
routing modes (`ask_before_crossing` default); cloud-crossing confirmation;
streaming + cancellation; provider/model display; per-response metadata;
request audit events; provider change never erases identity/conversations/
knowledge/memory (§13).

## R3 — Structured knowledge (Phase 3)

Knowledge record schema (§17) with all required fields/types/states;
stored-fact vs sourced-claim vs approved-conclusion vs inference vs temporary
context vs memory-candidate vs correction vs supersession (§17); contradiction
sets preserve both claims (§18); corrections preserve history (§18); tombstones
not in current retrieval; What I Know filters (§33).

## R4 — Secure source ingestion (Phase 4)

File picker + revocable scoped folder grants (§27); encrypted source storage +
checksums; txt/md/pdf/docx/json/csv + png/jpeg/webp; OCR adapter (§23);
resumable import jobs + cancellation + failure reporting; no macro/active
execution; no plaintext temp files left behind (§22).

## R5 — Retrieval and citations (Phase 5)

FTS5 + metadata/vault/permission/sensitivity/domain/recency filters + ranking +
dedup + context budgeting; embedding + vector interfaces + local adapter +
hybrid ranking; 11-step retrieval process (§24); citation precision + trust
classification; reindexing; deleted-source removal; cross-vault impossible;
local-only never crosses to cloud (§24, §25).

## R6 — Corpus import/export (Phase 6)

Versioned schemas + manifests; full/incremental export; stable IDs; idempotent
re-import; conflict detection; updates/supersession/tombstones; import history;
index rebuild; CLI validator; representative fixtures (§20, §21).

## R7 — Auditable memory (Phase 7)

Memory-candidate workflow with full provenance fields; approve/edit/reject/
defer/temporary/correct/supersede/delete; rejected never in retrieval; deferred
stay candidates (§26).

## R8 — Permission architecture (Phase 8)

Tool registry + permission records; action preview + confirmation; allow-once /
session / narrow-scope; revocation immediate; read-only/draft-only/mock tools;
no tool crosses vault boundaries; always-require-approval list (§28, §29).

## R9 — Product validation (Phase 9)

Windows packaging + clean-machine install; upgrade migration; full + recovery
backup restore; corrupted-backup + wrong-passphrase handling; retrieval quality;
provider-switch personality; security regression; threat-model review;
performance at scale; failure/interruption recovery; user + developer docs;
dependency license report (§35, §38).
