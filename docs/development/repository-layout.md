# Repository layout

[Architecture](../architecture/overview.md) · [Maintainer guide](MAINTAINER-GUIDE.md)

| Area | Purpose | Start here when changing… |
| --- | --- | --- |
| `src/views/` | Eight React product views | Chat, Vaults, Sources, Settings, Memory, Audit, Backup UI |
| `src/lib/tauri.ts` | Typed frontend command wrappers | Command arguments/results |
| `src-tauri/src/app.rs` | Tauri command surface and active-vault coordination | UI/backend integration |
| `src-tauri/src/chat.rs`, `providers.rs`, `settings.rs` | Prompt routing, adapters, provider configuration | Model/provider behavior |
| `vault.rs`, `crypto.rs`, `db.rs`, `lock.rs` | Keys, SQLCipher lifecycle, migrations, locking | Vault/security behavior |
| `knowledge.rs`, `corpus.rs`, `memory.rs` | Canonical knowledge and review states | Personal corpus semantics |
| `sources.rs`, `retrieval.rs`, `citations.rs` | Extraction, FTS/vector retrieval, provenance output | Ingestion and search |
| `backup.rs` | Encrypted package format and restore validation | Backup/recovery internals |
| `permissions.rs`, `tools.rs`, `audit.rs` | Capability gates and records | Actions/privacy audit |
| `src-tauri/tests/` | Workflow, forbidden behavior, lifecycle, live provider tests | Cross-module validation |
| `schemas/` | Corpus JSON schemas | Interchange compatibility |
| `src-tauri/tauri.conf.json` | Window, CSP, metadata, bundle targets | Packaging |

Root architecture/status documents are canonical product records. Focused
guides live under `docs/`; keep both aligned when behavior changes.
