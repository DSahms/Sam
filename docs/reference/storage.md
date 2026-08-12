# Storage reference

[Vault architecture](../architecture/vault-architecture.md) · [Security model](../security/security-model.md)

The default root is `%LOCALAPPDATA%\app.sammy.desktop`. Each vault is under
`vaults/<vault-id>/`:

```text
vaults/<id>/
├── vault.json   # non-secret metadata and wrapped keys
└── vault.db     # SQLCipher canonical data, settings, audit, FTS, vectors
```

Original source bytes are AES-GCM blobs inside the encrypted database. The
vector index is a table in the same per-vault SQLCipher boundary, not a shared
external database. Backup destinations are selected by the owner and are not
uploaded by Sammy.

Do not hand-edit, sync partially, or copy a live vault directory as a replacement
for the authenticated backup workflow.
