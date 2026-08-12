# Provider architecture

[Provider guide](../user-guide/providers.md) · [Existing deep design](../PROVIDER_ARCHITECTURE.md)

The Rust `Provider` abstraction exposes metadata, model listing, health, and chat
completion. Transport traits make KoboldCpp and Venice deterministic to test
without credentials or network access.

The current implementation is synchronous request/response. KoboldCpp uses a
configurable base URL. Venice pins scheme, host, port, and `/api/v1` path before
attaching authorization. Routing consumes mode, local/cloud roster, health, and
confirmation; cloud eligibility is not inferred from model text.

To add a provider, implement the backend trait, extend configuration and roster
construction, preserve the `crossed_to_cloud` contract, add error sanitization,
and test routing/privacy before exposing settings. See [adding a provider](../development/adding-a-provider.md).
