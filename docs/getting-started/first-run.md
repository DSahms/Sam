# First run

[Documentation home](../README.md) · [Vault guide](../user-guide/vaults.md)

Sammy starts on **Vaults**. Private commands remain unavailable until a vault is unlocked.

1. Create a vault using Personal, Witness, Consigliere, or Custom as a template.
2. Save the one-time recovery code outside the device.
3. Confirm it has been saved. Creation intentionally does not auto-unlock.
4. Unlock with the master passphrase and review the inactivity-lock policy.
5. Configure a provider in **Settings**.

Recovery verifies the code and replaces a forgotten passphrase without changing
the vault data key or existing recovery wrapping.

> [!IMPORTANT]
> Sammy has no vendor support key. Losing both credentials makes the vault
> unrecoverable by design.
