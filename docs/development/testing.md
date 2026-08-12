# Testing

[Build](build.md) · [Release checklist](release-process.md#release-checklist)

## Fast feedback

```powershell
cargo test -p sammy vault::tests
npm test -- --run
npm run typecheck
```

## Full automated gate

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
npm test -- --run
npm run typecheck
npm run lint
npm run format:check
npm audit --audit-level=low
npm run build
npm run tauri:build
```

Rust unit tests sit beside modules. `e2e_workflows.rs` crosses subsystem
boundaries; `forbidden_behaviors.rs` proves privacy/security negatives;
`vault_lifecycle.rs` covers key lifecycle and isolation; provider and ingestion
modules contain deterministic transport/extractor coverage. Frontend Vitest tests
cover command wrappers, navigation, and Backup & Recovery.

`live_koboldcpp.rs` is the only environment-dependent provider test. With an API
at `localhost:5001`, run:

```powershell
cargo test -p sammy --test live_koboldcpp -- --nocapture
```

The last release pass recorded 213 Rust and 12 frontend tests. Test counts are a
checkpoint, not an invariant; zero failures and complete gate coverage matter.
