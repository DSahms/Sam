# Threat model

[Security model](security-model.md) · [Canonical threat model](../../THREAT_MODEL.md)

| Threat | Implemented mitigation | Residual boundary |
| --- | --- | --- |
| Stolen powered-off computer/copied vault | SQLCipher, wrapped random key, Argon2id, no plaintext credential | Weak passphrases remain guessable; OS metadata exposes vault existence/name |
| Exposed API credential | Key encrypted under vault key; never returned to UI/logs; Venice endpoint pinned | An unlocked compromised OS/provider account can still expose it |
| Accidental cloud routing | Routing modes, confirmation, `local_only` filtering, audit | Owner-approved cloud context leaves the device |
| Malicious ingested content | Local non-executing extractors; DOCX macros ignored; unsupported input fails | Extracted text may influence model output and must be treated as untrusted data |
| Corrupt backup | Authenticated encryption, format/version checks, path validation | No recovery if all valid copies and credentials are lost |
| Accidental restore | Preview and explicit replacement confirmation | A confirmed replacement is destructive to the prior same-ID directory |
| Crash during write/restore | Atomic manifest write; staging then rename | Filesystem/hardware failure can still require backup recovery |
| Provider unavailable | Sanitized failure, deterministic non-cloud fallback, corpus remains local | No real-model response until service recovers |

See the root threat model for detailed assumptions and non-goals.
