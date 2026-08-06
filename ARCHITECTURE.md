# Sammy — Architecture

This document describes the system architecture. Per-subsystem detail lives in
`docs/`. Implementation status lives in `STATE.md`.

## Layered boundaries (directive §7)

```
React User Interface
        ↓
Tauri Command and Event Boundary
        ↓
Rust Application Services
        ↓
Agent and Conversation Runtime
        ↓
Knowledge, Corpus, Retrieval, and Permission Interfaces
        ↓
SQLCipher Vault and Encrypted Source Store
        ↓
Replaceable AI Providers, Extractors, and Index Engines
```

The Tauri command/event boundary is the trust frontier: the frontend receives
only the *results* of operations, never vault keys, never the unlocked database
handle, never provider credentials.

## Module map (Rust, `src-tauri/src/`)

| Module         | Owns                                            | Phase |
| -------------- | ----------------------------------------------- | ----- |
| `crypto`       | Argon2id, AES-256-GCM, CSPRNG, key wrap         | 1     |
| `vault`        | Create / unlock / lock / multi-vault isolation  | 1     |
| `audit`        | Append-only audit events                        | 1+    |
| `settings`     | Encrypted per-vault + app settings              | 1+    |
| `backup`       | Encrypted backup & recovery                     | 1 / 9 |
| `identity`     | Provider-independent companion identity         | 2     |
| `conversation` | Conversation & message model                    | 2     |
| `providers`    | Mock / KoboldCpp / Venice adapters + routing    | 2     |
| `knowledge`    | Curated records, states, contradictions         | 3     |
| `sources`      | Encrypted ingestion, extractors, OCR            | 4     |
| `retrieval`    | FTS5 + vector + hybrid ranking                  | 5     |
| `citations`    | Citation precision + trust classification       | 5     |
| `corpus`       | Versioned import/export packages                | 6     |
| `memory`       | Auditable memory-candidate workflow             | 7     |
| `permissions`  | Tool registry, grants, revocation               | 8     |
| `tools`        | Read-only / draft / mock tools (no execution)   | 8     |
| `ids`          | Stable typed identifiers (VaultId, RecordId...) | shared|
| `error`        | Sanitized error type (no secret leakage)        | shared|
| `app`          | Process state, unlocked-vault session           | shared|

## Hard boundaries

- Provider-specific behavior stays inside provider adapters.
- Extractor-specific behavior stays inside extractor adapters.
- Vector-engine-specific behavior stays behind the retrieval interface.
- The canonical knowledge store is SQLCipher, **never** a vector index.
- React never accesses database files or receives vault encryption keys.
- Imported source content is untrusted data; model output is never canonical.

## Frontend (`src/`)

React + TypeScript (strict). The eight primary views (directive §33):
Vaults, Chat, What I Know, Sources, Memory Review, Privacy & Audit, Backup &
Recovery, Settings. Visual state lives here; the source of truth for vault
state, permissions, crypto state, provider credentials, and audit state is
always Rust.

## Cryptographic spine (Phase 1)

Per `docs/KEY_MANAGEMENT.md`: a random vault data-encryption key is wrapped by
an Argon2id-derived key-encryption key (from the master passphrase) and
wrapped again separately by a recovery wrapping key. SQLCipher is keyed by the
unwrapped data-encryption key. Source files and backups use AES-256-GCM.

## Persistence

SQLite via SQLCipher, bundled and statically linked with vendored OpenSSL, so
no system SQLite/OpenSSL install is required. Versioned migrations. Atomic
file replacement for source files and backups, with checksums and
authenticated metadata.

## Distribution

Tauri 2 bundles MSI and NSIS installers for Windows 11 x64. First release is a
private, closed-source build designed to be commercially distributable later.
Dependencies are chosen for license compatibility with future commercial
distribution (see `EXTERNAL_BLOCKERS.md` for the license report status).
