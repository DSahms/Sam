# First run

[Documentation home](../README.md) · [Vault guide](../user-guide/vaults.md)

Sammy starts on **Vaults**. Private commands remain unavailable until a vault is unlocked.

A vault is Sammy's encrypted container for one body of identity, knowledge,
conversations, sources, settings, and history. Separate vaults cannot retrieve
from one another.

1. Create a vault using Personal, Witness, Consigliere, or Custom as a template.
2. Save the one-time recovery code outside the device.
3. Confirm it has been saved. Creation intentionally does not auto-unlock.
4. Unlock with the master passphrase and review the inactivity-lock policy.
5. Configure a provider in **Settings**.

Success looks like a green unlocked status badge identifying the active vault.
Settings, Chat, Sources, and other private views can then access that vault.

Recovery verifies the code and replaces a forgotten passphrase without changing
the vault data key or existing recovery wrapping.

> [!IMPORTANT]
> Sammy has no vendor support key. Losing both credentials makes the vault
> unrecoverable by design.
