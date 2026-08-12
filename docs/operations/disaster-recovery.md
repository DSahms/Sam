# Disaster recovery

[Backup procedure](backup-and-restore.md) · [Vaults](../user-guide/vaults.md)

Maintain multiple encrypted backup packages on media separate from the primary
computer. Store the master passphrase and recovery code separately from each
other and from the backup. Periodically preview and test restore using a
disposable environment.

On a replacement computer, install Sammy, select the package in **Backup &
Recovery**, preview, restore using either credential, and verify data. The old OS
account, keychain, and network are not required.

There is no support-assisted unwrap path. If every backup is corrupt/lost and the
original vault is unavailable—or both credential methods are lost—recovery is
impossible by design.
