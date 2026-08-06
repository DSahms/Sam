# Sammy — State

This file describes **only what currently works**, with evidence. It is updated
every checkpoint. If something is not here, it does not work yet.

Last updated: 2026-08-05

## Works now

### Foundation (Phase 0 — complete and verified)
- **Workspace builds.** `cargo build --workspace` succeeds on Windows 11 x64
  with the MSVC toolchain. SQLCipher + OpenSSL are bundled and compiled from
  source via `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature.
- **Frontend builds.** `npx vite build` produces a production bundle
  (~148 KB JS / ~2 KB CSS) in `dist/`.
- **Rust tests pass.** `cargo test --workspace` → 3 passed, 0 failed
  (typed-id round-trip / rejection / uniqueness).
- **Frontend tests pass.** `npx vitest run` → 3 passed, 0 failed (Sidebar
  renders all eight destinations, marks active, notifies on select).
- **Quality gate (partial).** `cargo fmt --all -- --check` passes;
  `cargo clippy --workspace --all-targets` is clean (0 warnings);
  `npx eslint . --max-warnings=0` passes; `npx prettier --check` passes;
  `npx tsc -p tsconfig.app.json --noEmit` passes.
- **App runs.** `sammy` Tauri binary compiles and registers two commands
  (`ping`, `app_meta`); the React shell renders the sidebar + 8 views and
  calls `app_meta` to show version + lock state.
- **CLI binary.** `sammy-cli --version` / `--help` work; `corpus validate`
  is a stub until Phase 6.
- **Module boundaries.** All §7 subsystems exist as Rust modules (`crypto`,
  `vault`, `audit`, `identity`, `conversation`, `providers`, `knowledge`,
  `corpus`, `sources`, `retrieval`, `citations`, `memory`, `permissions`,
  `tools`, `backup`, `settings`) plus shared `ids`, `error`, `app`.
- **Docs.** All canonical project documents and `docs/` deep-dives exist.

### Vault spine (Phase 1 — in progress)
- Nothing in Phase 1 is wired yet. `crypto`, `vault`, `audit`, `backup`,
  `settings` modules contain only their scope documentation. The `AppState`
  holds a placeholder `Option<VaultId>`; no real unlock exists.

## Known limitations / mocks in use

- All Phase 1+ subsystems are stubs. No real vault, crypto, DB, audit, or
  backup behavior exists yet.
- The eight UI views are placeholders describing their implementing phase.
- No code-signing; builds produce unsigned dev packages.
- npm dev-dependency audit advisories exist (dev-only, not shipped).

## External blockers

See `EXTERNAL_BLOCKERS.md`: build PATH must include Strawberry Perl (present);
no code-signing cert; no Venice API key; no KoboldCpp endpoint; OCR binding not
finalized; license report not yet generated. None block the build or tests.

## Next work

Begin Phase 1's first vertical slice: the `crypto` module (CSPRNG, Argon2id,
AES-256-GCM, key wrap) with unit tests, followed by the `vault` module
(create/unlock/lock) over a real SQLCipher database.
