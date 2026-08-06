# Corpus Import / Export Contract

This document defines the on-disk package format and the behavioral contract
for moving corpus data into and out of a vault. It governs backups, manual
exports, machine-to-machine migration, and any future sync feature. Everything
in this document applies **per vault**; there is no such thing as a
multi-vault package.

## Package overview

An export package is a single opaque, encrypted archive written to a
user-selected location. Inside the encryption envelope it contains:

```
package/
  manifest.json           # package metadata + schema versions + checksums
  sources/                # Layer 0: encrypted raw source files
  corpus.sqlite.enc       # Layer 1: encrypted SQLCipher snapshot (or delta)
  indexes/                # Layer 2: optional prebuilt indexes (rebuildable)
  recovery-instructions.txt  # human-readable, non-private restore steps
```

Indexes are optional and always rebuildable; a consumer may ignore them and
rebuild from Layer 1 on first unlock.

## Stable record IDs

Every knowledge record, source, conversation, message, and memory entry has a
stable, content- and vault-scoped identifier:

- **IDs are immutable.** A record keeps the same ID across every export and
  re-import for the lifetime of the vault.
- **IDs are vault-scoped.** They are derived from the originating vault ID so
  that an import into a *different* vault can detect a duplicate without
  collision.
- **IDs are never reused.** Deleted records leave tombstones; their IDs are
  retired permanently.

## Schema versions

The manifest declares:

- `manifest_schema_version` — the format of the package itself.
- `corpus_schema_version` — the SQLCipher schema version inside `corpus.sqlite.enc`.
- `app_version` — the Sammy version that produced the package.

An importer refuses a package whose `manifest_schema_version` it does not
understand. It will apply forward migrations on `corpus_schema_version` only
along supported upgrade paths; otherwise it errors out with a clear message.

## Full vs incremental export

- **Full export** contains every Layer 1 record, every source, every
  tombstone, and the full conversation history. It is self-sufficient for a
  bare-metal restore.
- **Incremental export** carries only records, sources, tombstones, and
  conversations changed since a watermark (a monotonic sequence number or a
  last-export timestamp recorded in the manifest). Incremental packages name
  the prior package they build on; an importer will not apply an incremental
  whose base is unknown or has gaps.

## Record lifecycle states in a package

A package records each item in one of these states so that the importer can
apply changes correctly:

- **new** — record did not exist in the destination.
- **updated** — same ID, newer `updated_at` / sequence number; replaces prior
  content after conflict check.
- **superseded** — replaced by a newer record ID; carries a `superseded_by`
  pointer so old citations can be rewritten.
- **tombstone** — record was deleted in the source; importer records the
  tombstone and removes the live row.

## Manifest validation

Before any data is read, the importer validates the manifest:

1. Parse `manifest.json`; reject on malformed JSON.
2. Check `manifest_schema_version` is supported.
3. Verify the manifest's HMAC over the file list (tamper detection).
4. Verify per-file checksums (size + SHA-256) match the bytes on disk.
5. Check `corpus_schema_version` is migratable to the local version.
6. Decrypt the corpus snapshot using the unlock credential; reject on MAC
   failure.

Only after all six checks pass does the importer begin mutating destination
state.

## Source references

Records reference sources by stable source ID plus an optional byte/character
range. The package must carry every source a referenced record points to, or
the importer must record a *dangling reference* and refuse to mark those
records as fully imported. A package that references a missing source fails
manifest validation with `MISSING_SOURCE_REF`.

## Conflict detection

On import, for each record the importer compares `(id, version,
updated_at, content_hash)`:

- If the destination has no such ID → **insert** as new.
- If destination version equals source version and hashes match → **no-op**.
- If destination `updated_at` is older → **update** with source.
- If destination `updated_at` is newer than source → **conflict**: default to
  keeping destination, log to import history, surface to the owner for
  resolution. Importer must not silently overwrite.
- If destination row is a tombstone → **skip** (deletions are sticky).

## Import history

Every import writes an entry to the vault's `import_history` table:

- package path, package hash, package manifest version
- source vault ID
- timestamp, importing app version
- counts: inserted, updated, superseded, tombstoned, conflicts, skipped
- outcome: `complete | partial | aborted`

Import history is itself part of the corpus and is exported in subsequent
packages; it is the audit trail for how knowledge entered the vault.

## Idempotent re-import

Importing the **same package twice** must produce no duplicates and no
second application of changes. This is enforced by:

1. Recording the package hash in `import_history` on first import.
2. On subsequent import, detecting the known hash and short-circuiting to a
   no-op (still logged as a repeat import).
3. Even if forced, per-record conflict detection (above) makes the second
   pass a no-op because every record's `(version, hash)` already matches.

This also covers re-importing a *full* package after an *incremental*: a full
package replayed against a vault that already contains its incremental
descendants must reduce to inserts of only the records not yet present.

## Runtime-index rebuild after import

Once Layer 1 is reconciled, the importer:

1. Marks the FTS5 and vector indexes stale for every inserted/updated/
   superseded/tombstoned record.
2. Schedules a runtime-index rebuild (or an incremental reindex of just the
   affected IDs).
3. Blocks the import transaction from returning `complete` until the rebuild
   is queued and durable.

Indexes are never part of the import conflict-resolution contract. Two vaults
with identical Layer 1 may have entirely different Layer 2 and still be
considered reconciled.
