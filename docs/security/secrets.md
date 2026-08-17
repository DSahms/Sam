# Secret handling

[Security model](security-model.md) · [Storage reference](../reference/storage.md)

Passphrases and recovery codes enter Rust commands only for the requested key or
backup operation. Plaintext values are not written to manifests, audit events, or
provider errors. The recovery code itself is shown once at creation and is never
stored; only its independent wrapped-key material remains.

Venice API keys are AES-GCM encrypted with the active vault key. Configuration
reads reveal only whether a key exists. Do not add secret fields to frontend DTOs,
debug formatting, snapshots, fixtures, or logs.

Windows code-signing material is a separate class of secret. Never commit `.pfx`,
`.p12`, `.pvk`, `.spc`, or a `.signing/` directory. Never put a PFX password or
cloud-signing key in `tauri.conf.json`, `STATE.md`, or build logs. A certificate
**thumbprint** is an identifier, not the private key, but still review it before
it lands in git. See [Windows code signing](../development/windows-code-signing.md).
