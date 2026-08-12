# Adding a provider

[Provider architecture](../architecture/provider-architecture.md) · [Testing](testing.md)

1. Implement the Rust provider/transport abstraction in `providers.rs`.
2. Classify it explicitly as local or cloud and set `crossed_to_cloud` correctly.
3. Add vault-scoped configuration in `settings.rs`; encrypt credentials with the
   active vault data key and never return them to React.
4. Extend roster construction in `chat.rs` without changing corpus ownership.
5. Add Settings controls and typed command wrappers.
6. Add deterministic transport tests for model listing, chat, timeout, auth,
   malformed response, and prompt/secret redaction.
7. Add routing tests proving no silent cloud crossing and correct consent/audit.

Do not encode provider-specific data into canonical knowledge records or make a
vector service authoritative. Live credentials are a validation layer, not a
substitute for deterministic tests.
