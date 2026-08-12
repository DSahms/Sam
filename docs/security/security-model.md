# Security model

[Threat model](threat-model.md) · [Secrets](secrets.md) · [Architecture boundaries](../architecture/security-boundaries.md)

Sammy protects local data with independent encrypted vaults. A random vault data
key opens SQLCipher and protects source/configuration secrets; it is wrapped
separately under passphrase- and recovery-derived keys. Argon2id slows passphrase
guessing. Keys and database handles remain in Rust and are dropped on lock.

Provider routing is a data-egress boundary. Local-only records are filtered before
cloud context; Venice requires its pinned HTTPS endpoint and an encrypted
vault-scoped key; cloud decisions are audited. Backup packages have independent
encryption and authenticated integrity validation.

Sammy assumes the OS, executable, and selected model service are trustworthy at
the moment an unlocked vault is used. It does not prevent an administrator,
malware, keylogger, memory scraper, or screen recorder on that machine from
observing live use. Unsigned packages provide no publisher authenticity until a
certificate is supplied.
