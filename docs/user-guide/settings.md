# Settings

[Documentation home](../README.md) · [KoboldCpp setup](../getting-started/koboldcpp.md) · [Provider reference](providers.md)

Settings configures enabled providers, endpoints, models, and local/cloud
routing inputs for the active vault. Test a connection before saving a selected
model.

KoboldCpp endpoints are configurable and intended for localhost. Venice uses the
fixed production base URL `https://api.venice.ai/api/v1`; Sammy refuses to send
its credential to another host or path. The Venice key is encrypted with the
active vault key, is never returned to the frontend, and an empty replacement
removes it.

Connection testing is diagnostic; normal Chat uses the saved configuration.
Settings require an unlocked vault because configuration and credentials are
vault-scoped.

## Personal knowledge (PKC)

PKC is a **separate** durable knowledge system. It is not the records in
**What I Know**. See [external PKC setup](../getting-started/external-pkc.md).

The Settings card lets you enable or disable read-only access, set the PKC
location and source identity, discover local defaults, and **Test connection**.
**Find local defaults** does not embed another PC's drive letters; choose the
folders on this computer. Consumer identity is `sammy`; purpose is
`personal_consigliere`. Evidence never leaves this machine: cloud chats skip PKC.
A green **Authorized** badge means Sammy actually verified the connection, not
that a checkbox was ticked.

