# Sammy — Security Model

How the threat boundary in `THREAT_MODEL.md` is actually enforced. This is the
mechanism document; the *claim* document is the threat model.

## Cryptographic primitives (Phase 1)

| Purpose                        | Primitive                       |
| ------------------------------ | ------------------------------- |
| Password-based key derivation  | Argon2id                        |
| Vault data-encryption key      | 256-bit random (CSPRNG)         |
| Symmetric encryption           | AES-256-GCM (96-bit nonce)      |
| Database encryption            | SQLCipher (AES-256, keyed by DEK)|
| Secret comparison              | constant-time                   |
| Secret zeroing                 | `zeroize` on drop               |

See `docs/KEY_MANAGEMENT.md` for the full wrapping hierarchy.

## Key custody

- The vault data-encryption key is generated locally with the CSPRNG and never
  leaves the device in plaintext.
- It is wrapped by an Argon2id key-encryption key (from the master passphrase)
  and wrapped separately by a recovery wrapping key (from a high-entropy
  recovery key shown once at vault creation).
- The plaintext passphrase, plaintext vault key, and any reversible password
  hint are **never** stored.
- Provider credentials are encrypted at rest; the frontend never receives them.

## Access control

- No private access, chat access, or source access before a vault is unlocked.
- Vaults are fully isolated: separate DEKs, separate SQLCipher databases,
  separate source stores, separate indexes, separate audit. No cross-vault
  retrieval, no shared private embedding index.
- Locking clears active sensitive state where practical; default inactivity
  lock is 15 minutes, and the vault always locks on app exit, Windows session
  lock, and explicit Lock Vault.

## Data minimization & routing

- Provider routing modes include `local_only`, `prefer_local`, `prefer_cloud`,
  `cloud_only`, and `ask_before_crossing` (the default).
- Local-only data never goes to a cloud provider; provider failure never
  silently crosses a privacy boundary.
- Cloud requests carry only necessary context; every cloud transmission is an
  audit event; provider errors never log the full private prompt.

## Untrusted data & model output

- Imported sources are untrusted data: no macro/script/active-content
  execution. Extractor behavior stays inside replaceable adapters.
- Model output is never automatically canonical knowledge. AI proposes; the
  owner approves. Conversation content never becomes durable memory without
  review.

## Tool actions & permissions

- The first release executes no external actions. The permission architecture
  (Phase 8) gates every future tool action behind explicit, revocable,
  narrowly-scoped consent. No conversational request becomes permanent
  authorization; revocation is immediate.

## Deletion & immutability

- Active deletion is explicit and auditable: removes from views, lexical and
  vector indexes, derived chunks, cached prompt material, and derived
  summaries; leaves a minimal tombstone (no private content) so sync does not
  restore it.
- Audit records retain only ID, operation, timestamp, actor, optional reason —
  never deleted content.
- Encrypted backup files are immutable historical packages; deleting active
  data does not silently modify earlier backups.

## Honest limits

The mechanisms above defend the boundary in `THREAT_MODEL.md`. They do not
defeat an attacker with kernel/admin access to an unlocked machine, malware
already running as the owner, a compromised OS, hardware keyloggers, physical
coercion, or privileged screen capture. These limits are stated to the user,
not hidden.
