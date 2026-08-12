# Secret handling

[Security model](security-model.md) · [Storage reference](../reference/storage.md)

Passphrases and recovery codes enter Rust commands only for the requested key or
backup operation. Plaintext values are not written to manifests, audit events, or
provider errors. The recovery code itself is shown once at creation and is never
stored; only its independent wrapped-key material remains.

Venice API keys are AES-GCM encrypted with the active vault key. Configuration
reads reveal only whether a key exists. Do not add secret fields to frontend DTOs,
debug formatting, snapshots, fixtures, or logs.
