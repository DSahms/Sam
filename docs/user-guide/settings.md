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
