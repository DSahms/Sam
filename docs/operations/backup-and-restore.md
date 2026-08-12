# Backup and restore procedure

[User guide](../user-guide/backup-recovery.md) · [Disaster recovery](disaster-recovery.md)

## Backup

1. Unlock the intended vault and open **Backup & Recovery**.
2. Choose an owner-controlled destination ending in `.sammy-backup`.
3. Enter the matching master passphrase and recovery code.
4. Create the package, then use restore preview to confirm its ID/name/version.
5. Copy it to separate protected media and retain both credential methods safely.

## Restore

> [!WARNING]
> A confirmed restore replaces a vault with the same ID. Back up current data first.

1. Select the package and preview it.
2. Confirm that its identity is the intended vault.
3. Choose passphrase or recovery code and enter it.
4. Select the explicit replacement confirmation and restore.
5. Unlock and verify representative conversations, records, and sources.

Never rename extracted package contents or attempt a partial manual restore.
