# Sammy — Decisions

Material implementation decisions, recorded per directive §6. Each entry
includes the decision, reason, alternatives, security/data consequences,
reversibility, date, and affected files.

---

## D001 — Workspace layout: one workspace, one member crate (Phase 0)
- **Decision:** Cargo workspace with a single member crate `sammy` at
  `src-tauri/`; subsystems are *modules* within that crate, not separate crates.
- **Reason:** Keeps the Phase 0–1 build simple while preserving the workspace
  shape so any subsystem can be split into its own crate later without churn.
- **Alternatives:** One crate per subsystem now; a single flat crate without a
  workspace.
- **Security consequences:** None. Module visibility still enforces boundaries.
- **Data consequences:** None.
- **Reversibility:** High — moving a module to its own crate is mechanical.
- **Date:** 2026-08-05.
- **Affected:** `Cargo.toml`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`.

## D002 — SQLCipher via bundled-sqlcipher-vendored-openssl (Phase 0/1)
- **Decision:** Use `rusqlite` with the `bundled-sqlcipher-vendored-openssl`
  feature, compiling SQLCipher **and** OpenSSL from C source.
- **Reason:** Removes any system SQLite/OpenSSL dependency — essential for an
  offline, per-user Windows app and a clean-machine install. `bundled-sqlcipher`
  alone still needed a system OpenSSL (`OPENSSL_DIR`); vendoring it removes that.
- **Alternatives:** `bundled-sqlcipher` + a prebuilt system OpenSSL (install
  burden, license-distribution complexity); pure-SQLite without SQLCipher
  (rejected: violates the encryption-first requirement).
- **Security consequences:** Positive — the crypto library is built from
  auditable source and statically linked; no DLL-hijacking surface from a
  system OpenSSL.
- **Data consequences:** None.
- **Reversibility:** Medium — swapping the crypto backend later touches the
  build but not the SQLCipher usage above it.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/Cargo.toml`, `EXTERNAL_BLOCKERS.md`,
  `docs/DEVELOPMENT_RUNBOOK.md`.

## D003 — Build requires Strawberry Perl on PATH (Phase 0)
- **Decision:** Document Strawberry Perl as a build prerequisite (it is already
  installed at `C:\Strawberry\`). Builds must place
  `C:\Strawberry\perl\bin` and `C:\Strawberry\c\bin` ahead in `PATH`.
- **Reason:** Vendored OpenSSL's `Configure` needs Perl modules
  (`Locale::Maketext::Simple`, `Params::Check`, `IPC::Cmd`) that the Perl
  bundled with Git for Windows lacks. Strawberry Perl has them.
- **Alternatives:** Install a different complete Perl; avoid vendored OpenSSL.
- **Security consequences:** None.
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `EXTERNAL_BLOCKERS.md`, `docs/DEVELOPMENT_RUNBOOK.md`,
  `README.md`.

## D004 — Typed id newtypes over Uuid (Phase 0)
- **Decision:** `VaultId`, `SourceId`, `RecordId`, `ConversationId`,
  `MessageId`, `MemoryCandidateId`, `AuditEventId` are distinct newtypes
  wrapping `Uuid` (v4). String form is the canonical hyphenated UUID, preserved
  verbatim across exports/backups.
- **Reason:** Directive §4 requires stable typed identifiers; distinct types
  prevent mixing a `RecordId` with a `SourceId` at compile time.
- **Alternatives:** Plain `Uuid` everywhere; string IDs.
- **Security consequences:** None.
- **Data consequences:** Stable string IDs are the cross-package contract.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/ids.rs`.

## D005 — Opaque `Database` error variant (Phase 0)
- **Decision:** `rusqlite::Error` maps to an opaque `AppError::Database` variant
  with no SQL detail, so database internals never reach the frontend or logs.
- **Reason:** Directive §8 (no accidental disclosure through logs) and §13
  (errors must not expose private prompt text). Internal callers log detail at
  debug level before converting.
- **Alternatives:** Transparent SQL error forwarding.
- **Security consequences:** Positive — reduces information leakage.
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/error.rs`.

## D006 — No network/shell/filesystem Tauri capabilities by default (Phase 0)
- **Decision:** The default capability grants only core window/event/webview/app
  permissions. No filesystem, shell, or HTTP scope is granted.
- **Reason:** Directive §27 (explicit file access only) and §32 (network only
  for configured providers). Capabilities are added per-feature as the
  permission architecture lands.
- **Alternatives:** Broad `fs:default`.
- **Security consequences:** Positive — least privilege.
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/capabilities/default.json`.

## D007 — `forbid(unsafe_code)` at crate root (Phase 0)
- **Decision:** The `sammy_lib` crate is compiled with `#![forbid(unsafe_code)]`.
- **Reason:** Directive §4/§8 favor defense in depth; forbidding `unsafe` keeps
  the security spine auditable. (Crypanalysis happens inside audited
  dependencies that may use `unsafe` internally; our own code does not.)
- **Alternatives:** `allow(unsafe_code)` with review.
- **Security consequences:** Positive.
- **Data consequences:** None.
- **Reversibility:** Medium.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/lib.rs`.
