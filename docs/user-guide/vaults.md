# Vaults

[Documentation home](../README.md) · [Security model](../security/security-model.md) · [Backup](backup-recovery.md)

A vault is an independent security and data boundary. It contains its own
SQLCipher database, wrapped data-encryption key, corpus, conversations, settings,
indexes, and audit history. Vaults do not share keys or retrieval tables.

Create a vault with a name, template, and passphrase. Save the recovery code
shown once. Unlock with the passphrase; lock manually or allow the configured
5/15/30/60-minute inactivity policy to lock it. Windows session lock is also
detected. Only one vault is active in a process.

For a forgotten passphrase, enter the recovery code plus a new passphrase. Sammy
atomically replaces only the passphrase-wrapped key and opens the unchanged vault.
Wrong credentials return a generic cryptographic error.

Common problems: an already-open vault must be locked before another is used;
eight characters is the minimum passphrase length; neither Sammy nor a vendor can
recover a vault when both credentials are lost.
