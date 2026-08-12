# Tauri command surface

[Repository layout](repository-layout.md) · [Security boundaries](../architecture/security-boundaries.md)

Commands are registered in `src-tauri/src/lib.rs`, implemented chiefly in
`app.rs`, and wrapped for React in `src/lib/tauri.ts`.

| Group | Commands |
| --- | --- |
| App/vault | `ping`, `app_meta`, `vault_create/list/unlock/unlock_recovery/recover_change_passphrase/lock/status`, lock policy |
| Backup | `vault_backup`, two restore commands, `vault_backup_preview` |
| Chat/identity | conversation list/create/messages, `chat_send`, identity get/save |
| Knowledge/retrieval | list/add/approve/reject/tombstone/correct/search, `vector_index_rebuild` |
| Sources/corpus | source import/list/extracted/delete/search, corpus export/import |
| Memory | list/approve/reject/defer/mark temporary/delete |
| Permissions/audit | tool registry, permission grant/list/revoke, audit list/counts |
| Providers | config get/save, Venice key/test, KoboldCpp test/diagnostic chat |

New commands must validate vault state and IDs in Rust, return sanitized
`AppError`, avoid secret DTO fields, be registered, gain a typed wrapper, and be
covered at the appropriate unit/workflow/security level.
