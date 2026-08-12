# Vault architecture

[Vault user guide](../user-guide/vaults.md) · [Key management](../KEY_MANAGEMENT.md) · [Storage reference](../reference/storage.md)

Each vault directory contains `vault.json` and `vault.db`. The manifest stores
non-secret metadata, Argon2id parameters and salt, plus separate encrypted
wrappings of one random 256-bit data-encryption key under the passphrase and
recovery-derived keys. It never stores either plaintext credential or plaintext
data key.

The data key opens SQLCipher and encrypts original source blobs and settings
secrets. Atomic temporary-file replacement protects manifests. Database
migrations run transactionally on unlock. Vault IDs, directories, keys, tables,
FTS indexes, and vector rows are independent.
