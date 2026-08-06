# Backup and Recovery

Sammy's backup system exists so that a single human, with their passphrase
or recovery key, can move their entire vault to a new computer and lose
nothing — without depending on the original OS account, the original
keychain, any online service, or the vendor. Everything below applies per
vault.

## Where backups may go

Backups are written **only** to a location the owner explicitly selects at
backup time:

- a local folder on the same machine,
- a removable drive (USB, external SSD),
- a network folder (SMB share, NAS),
- a cloud-synced folder the owner owns (e.g., a Dropbox / OneDrive
  directory mounted as a filesystem path).

Sammy never sends a backup to a vendor-operated endpoint. The backup path is
a filesystem path the owner chose; what syncs it (if anything) is the
owner's business.

## Package shape

A backup is a single **opaque, encrypted** package. Even when stored in a
cloud-synced folder, the sync service sees only ciphertext. Inside the
envelope the package contains:

- the encrypted SQLCipher data (Layer 1)
- the encrypted source files (Layer 0)
- the encrypted indexes (Layer 2) — optional, always rebuildable
- a `manifest.json`
- a human-readable, non-private `recovery-instructions.txt`

The package is decryptable only with the master passphrase or the recovery
key. The encryption envelope keys are derived from those credentials; there
is no separate backup key and no escrow.

## Manifest contents

`manifest.json` records:

- `app_version` — the Sammy version that produced the backup.
- `manifest_schema_version` — the package format version.
- `corpus_schema_version` — the SQLCipher schema version inside.
- `vault_id` — the originating vault's stable ID.
- `created_at` — creation timestamp (ISO 8601, UTC).
- per-file `checksums` — SHA-256 + size for every file in the package.
- `restore_requirements` — minimum Sammy version and any invariants
  required to restore (e.g., "requires recovery key or master passphrase").
- an HMAC over the file list to detect tampering.

## Human-readable recovery instructions

`recovery-instructions.txt` is intentionally non-private — it contains no
secrets, only the steps to restore the vault on a fresh machine:

1. Install Sammy.
2. Choose "Restore from backup".
3. Point Sammy at this package.
4. Enter the master passphrase *or* the recovery key.
5. Confirm the vault preview and let Sammy replace local state.

These instructions may be read by anyone; they are useless without the
passphrase or recovery key.

## Operations

### Full encrypted backup

Writes every Layer 0/1/2 file plus the manifest. Self-sufficient for a
bare-metal restore.

### Incremental backup (design)

Carries only files changed since a prior full or incremental backup, named
in the manifest. An incremental is restorable only together with its base;
the restore path verifies the chain before applying.

### Verification

After writing, Sammy re-reads every file in the package and verifies its
SHA-256 against the manifest. A backup that fails verification is flagged as
failed and must be re-run; it is not advertised as a successful backup.

### Restore preview

Before mutating any local state, Sammy reads the package, validates the
manifest, decrypts the corpus snapshot, and presents a **preview**: counts
of sources, records, conversations, the source vault ID, the schema
versions, and the package creation time. The owner confirms before any
local data is touched.

### Restore to a temp location

Restore writes to a temporary directory first. The live vault is replaced
only after the restored copy passes an integrity check (schema migration
applied, FTS5 rebuilt, source refs resolved, checksums verified).

### Wrong credential / corrupted / unsupported / interrupted

Each failure has a defined, user-visible handling:

- **Wrong passphrase or recovery key:** generic "incorrect" error; no
  partial state is written.
- **Corrupted package:** detected by checksum or HMAC mismatch; restore
  aborts; the temp directory is deleted; the live vault is untouched.
- **Unsupported version:** the manifest declares a version the running
  Sammy cannot read; restore refuses with a clear message naming the
  required version.
- **Interrupted restore:** detected on next launch via a marker file; Sammy
  discards the partial temp directory and reverts to the pre-restore
  vault. The owner is told the restore did not complete.

### Duplicate-vault detection

The restored vault's `vault_id` is compared against any locally present
vault. If the same `vault_id` is already present, Sammy refuses to silently
overwrite; the owner must explicitly confirm replacement, and Sammy offers
to back up the existing local vault first.

### User-controlled final replacement

Even after the restored copy passes integrity checks, the final act of
replacing the live vault is an explicit owner confirmation. Sammy never
overwrites the live vault as a side effect of opening a package.

## Cross-machine portability

A backup restored on a **replacement computer** works without:

- the original OS account,
- the original Windows keychain,
- any online service,
- the original device's TPM or Hello enrollment.

The master passphrase **or** the recovery key is sufficient. This is the
load-bearing property of the entire backup design; any feature that would
introduce a dependency on the original device (for example, a key wrapped
only by TPM) is rejected.

## Relation to import/export

Backup packages use the same envelope and manifest format described in
`CORPUS_CONTRACT.md`, with the backup-specific additions above (recovery
instructions, app version, restore requirements). A backup is effectively a
full self-contained export plus restore metadata; the import contract
(idempotent re-import, stable IDs, conflict detection) applies unchanged.
