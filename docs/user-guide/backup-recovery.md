# Backup and recovery

[Operations procedure](../operations/backup-and-restore.md) · [Vaults](vaults.md) · [Disaster recovery](../operations/disaster-recovery.md)

A `.sammy-backup` package contains the selected vault directory in an encrypted,
versioned container with integrity metadata. It can be opened with the backup
passphrase or its recovery code, without the original Windows account or an
online service.

To back up, unlock the vault, choose a destination, supply the passphrase and
recovery code, and run backup. Store the resulting package separately and test
its preview.

To restore, select a package, preview its vault ID/name/version, choose a
credential method, and enter the matching secret.

> [!WARNING]
> Restore can replace an existing vault with the same ID. Read the preview and
> select the explicit replacement confirmation only after protecting current
> data. Sammy rejects restore when confirmation is absent.

Restore decrypts into a sibling staging directory, validates paths and package
integrity, then replaces the destination by rename. Wrong credentials, corrupt
packages, traversal paths, unsupported format versions, and incomplete staging
fail safely. A backup does not protect secrets that have already been disclosed
outside Sammy or an unlocked machine controlled by malware.
