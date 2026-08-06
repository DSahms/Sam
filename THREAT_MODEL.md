# Sammy — Threat Model

This document states, honestly, what the first release defends against and what
it does not. Sammy does **not** claim perfect security, absolute anonymity,
zero knowledge, bank-grade security, or protection from an already-compromised
operating system.

## What the first release defends against (directive §8)

- Theft of a powered-off or locked computer.
- Copying the application's vault files from disk.
- Theft of an encrypted backup.
- Access by another ordinary local computer user.
- Accidental disclosure through logs.
- Accidental cloud transmission of local-only data.
- Cross-vault data leakage.
- Malicious or prompt-injected documents (treated as untrusted data; no macro
  or active-content execution).
- Database corruption.
- Interrupted writes / imports.
- Incorrect provider fallback (no silent cloud crossing).
- Accidental deletion (tombstones; immutable historical backups).
- Compromised or untrusted cloud AI providers, through data minimization and
  explicit routing.
- Unauthorized tool execution inside the application (permission gate).

## What the first release does NOT defend against

- Malware already running with the owner's OS permissions while the vault is
  unlocked.
- An attacker with administrator or kernel-level access to a running, unlocked
  computer.
- Physical coercion.
- Hardware keyloggers.
- A compromised operating system.
- Screen capture performed by other privileged software.
- Memory extraction by an administrator while the vault is actively unlocked.

Secure physical erasure from solid-state drives cannot be guaranteed; Sammy
does not claim otherwise.

## Credential recovery

A vault is recoverable only through an owner-controlled recovery key. If both
the master passphrase **and** the recovery key are lost, the vault is
permanently unrecoverable. There is no vendor backdoor and no support override.

## Where the boundary is enforced

- All cryptographic operations, vault state, provider credentials, and
  permissions live in Rust; the frontend never receives vault keys or the
  unlocked database handle.
- Every cloud transmission creates an audit event; local-only data never
  crosses to a cloud provider.
- All imported content is untrusted; model output is never canonical knowledge.
- Tool actions require explicit permission; none execute externally in the
  first release.

See `SECURITY_MODEL.md` for the mechanisms and `docs/KEY_MANAGEMENT.md`,
`docs/VAULT_MODEL.md`, and `docs/PERMISSION_MODEL.md` for subsystem detail.
