# Sammy — State

This file describes **only what currently works**, with evidence. It is updated
every checkpoint. If something is not here, it does not work yet.

Last updated: 2026-08-11 (Phases 0–8 + Phase 9 testing; checkpoint 90fc945)

## Works now

### Phase 0 — Foundation (verified)
- Tauri 2 + Rust + React + TS + Vite. `cargo build --workspace` and `vite build`
  succeed. SQLCipher + OpenSSL bundled. All §7 module boundaries as Rust modules.
- Quality gate: `cargo fmt`, `cargo clippy -D warnings`, eslint, prettier, tsc strict.
- `sammy-cli` binary with `corpus validate` subcommand.

### Phase 1 — Cryptographic vault spine (verified)
- `crypto`: SecretKey (zeroizing, constant-time eq), CSPRNG, Argon2id, AES-256-GCM,
  key wrap/unwrap, VaultKeyMaterial (passphrase + recovery unwrap), recovery codes.
- `vault`: per-vault dir (vault.json + vault.db); create/unlock/lock/list/delete;
  multi-vault isolation; SQLCipher encrypted-at-rest (tested); atomic writes.
- `db`: versioned, forward-only migrations with transactional rollback (v1–v5).
- `audit`: append-only audit_events API (record/list/count_by_category).
- `lock`: inactivity watchdog (5/15/30/60 min, manual); Windows session-lock
  detection (desktop-name polling via GetUserObjectInformationW); background thread.
- `backup`: SAMMYBK1 encrypted package format; create/restore (passphrase or
  recovery); integrity verification; path-traversal protection; staging + atomic
  rename.
- Tauri commands wired; functional Vaults UI (create/unlock/lock, recovery-code
  display, inactivity-policy picker).

### Phase 2 — Identity, chat, providers (verified)
- `identity`: structured CompanionIdentity (§12); PromptAssembly (8 sanitized
  sections); inspection_view (lengths only); per-vault store.
- `conversation`: conversations + messages model; create/append/list/messages.
- `providers`: Provider trait; MockProvider (deterministic), KoboldCppProvider
  (local, transport-abstracted), VeniceProvider (cloud, key-gated);
  resolve_routing (5 modes, default ask_before_crossing).
- `chat` runtime: orchestrates identity → prompt → routing → consent → provider →
  audit; cloud-crossing consent flow; denial → no transmission (audited).
- Tauri commands wired; functional Chat UI (conversations, send, routing picker,
  cloud-crossing consent banner with Approve/Deny).

### Phase 3 — Structured knowledge (verified)
- `knowledge`: 18 record types, 7 states, sensitivity, provenance, contradictions,
  corrections (supersession), tombstones; FTS5 search over current records only.
- Tauri commands wired; What I Know UI (add fact, search, status filters,
  approve/reject/correct/tombstone, contradiction/supersession badges).

### Phase 4 — Secure source ingestion (verified)
- `sources`: SourceKind classification; Extractor trait + Text/Markdown/JSON/CSV
  extractors; AES-256-GCM encrypted storage; import/list/extracted/delete/search.
- FTS5 query sanitization (phrase-quoting) in both sources and knowledge search.
- Tauri commands wired; Sources UI (file-picker import, search, preview, delete).

### Phase 5 — Retrieval and citations (verified)
- `citations`: CitationLocation, TrustClassification, ClaimLabel, ClaimBlock,
  AnnotatedAnswer with consistency check (no fabricated citation ids).
- `retrieval`: EmbeddingProvider + StubEmbeddingProvider; VectorIndex +
  InMemoryVectorIndex (cosine); retrieve() 11-step pipeline; assemble_answer().
- The vector index is rebuildable and is NOT the canonical corpus.

### Phase 6 — Corpus import/export (verified)
- `corpus`: CorpusManifest + CorpusRecord + CorpusPackage; export_full (read-only);
  validate_package (version, checksum, unique ids); import (idempotent upsert,
  tombstones); JSON round-trip. JSON schemas in schemas/.
- CLI: `sammy-cli corpus validate <package.json>`. Tauri commands wired.

### Phase 7 — Auditable memory (verified)
- `memory`: MemoryCandidate with full provenance; CandidateState
  (pending/approved/rejected/deferred/temporary); propose/approve (promotes to
  knowledge record)/reject/defer/mark_temporary/delete/list.
- Conversation content never becomes durable memory without review (§26).
- Tauri commands wired.

### Phase 8 — Permission architecture (verified)
- `permissions`: RiskLevel, Reversibility, ToolDeclaration, ToolRequest,
  PermissionMode; grant/check/revoke/list_active; revocation immediate.
- `tools`: registry of 5 tools (source_search, corpus_lookup, draft_generation,
  planning, mock_external); NO external actions execute.
- Tauri commands wired.

### Phase 9 — Product validation (in progress)
- **Forbidden-behavior test suite (§37):** 18 automated tests proving Sammy does
  NOT permit private access before unlock, wrong-passphrase success, cross-vault
  access, silent cloud fallback, tombstoned/deleted record retrieval, automatic
  memory approval, rejected memory retrieval, duplicate corpus imports, fake
  citation IDs, tool execution without permission, permanent auth from
  conversation, plaintext secrets in errors/Debug, corrupted backup restore,
  migration without rollback, mock cloud-crossing claims, or candidate records
  in current truth.
- **End-to-end workflow tests (§36):** 8 integration tests covering new-vault
  lifecycle, recovery (backup → restore on new registry via passphrase and
  recovery code), multiple vaults with isolation verification, source-grounded
  answers, corpus package round-trip with reimport-no-duplicates, memory
  workflow, provider privacy, and permissions.
- **Dependency license report:** `LICENSES.md` — 462 crates scanned, **no
  GPL/copyleft crates**; all permissive licenses.
- **Privacy & Audit UI** wired: audit summary by category + recent events table.
- **Memory Review UI** wired: candidate review queue with approve/edit/reject/
  defer/temporary/delete actions.

## Verified Phase exit criteria
- ✅ Private/chat/source access impossible before unlock.
- ✅ Wrong passwords fail safely; recovery-key restore works.
- ✅ Database content not plaintext-readable; vaults isolated.
- ✅ DB migration has rollback protection.
- ✅ Locking clears sensitive state; inactivity + session lock.
- ✅ Backup restores via passphrase or recovery; integrity verified.
- ✅ No silent cloud crossing; denial audited; no transmission.
- ✅ Provider change never erases identity/conversations.
- ✅ Facts/inference distinguishable; contradictions coexist; corrections preserve history.
- ✅ Tombstoned/deleted records excluded from retrieval.
- ✅ Corpus re-import creates no duplicates; tombstones remove availability.
- ✅ Memory requires review; rejected never in retrieval.
- ✅ No tool executes without permission; revocation immediate.
- ✅ FTS5 queries sanitized (no operator injection).

## Not yet done
- Phase 9 (product validation): Windows packaging/clean-machine install;
  performance tests at scale; dependency license report; user documentation.
- PDF/DOCX/image extractors + OCR adapter (interface ready; binding unchosen).
- Real KoboldCpp/Venice HTTP transports (interface ready; mock transports tested).
- Embedding/vector engine persistence (in-memory stub tested).

## External blockers
See `EXTERNAL_BLOCKERS.md`. None block the build or tests.

## Next work
Phase 9 product validation, or hardening the remaining adapters (PDF/DOCX/OCR,
real HTTP transports, persistent vector index).
