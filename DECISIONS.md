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

## D008 — Recovery code: 160 bits, base32 + checksum (Phase 1)
- **Decision:** Recovery codes are 20 random bytes (160 bits) from the CSPRNG,
  encoded as RFC 4648 base32 upper-case grouped in 4-char blocks, with one
  SHA-256 checksum byte appended for typo detection. The recovery KEK is
  HKDF-SHA256 of the entropy over the vault salt.
- **Reason:** 160 bits is comfortably beyond brute-force; base32 avoids the
  0/O/1/I ambiguity of base64 for human transcription; the checksum byte lets
  the UI flag a typo before a failed unwrap. HKDF binds the recovery key to the
  vault salt so the same code on a different vault (different salt) yields a
  different KEK.
- **Alternatives:** BIP-39 mnemonic words (more shareable, larger surface);
  raw hex (hard to transcribe); UUID format (shorter, less entropy).
- **Security consequences:** Positive — high entropy, typo-detectable, never
  stored. The recovery key is shown once and zeroizes on drop.
- **Data consequences:** The human form is the recovery contract; future
  versions must keep it parseable.
- **Reversibility:** Medium — format is versioned inside the wrapping design.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/crypto/recovery.rs`.

## D009 — Per-vault directory layout; manifest outside the DB (Phase 1)
- **Decision:** Each vault lives in `<app_data>/vaults/<vault_id>/` containing
  `vault.json` (the manifest + `VaultKeyMaterial`) and `vault.db` (SQLCipher).
  The manifest is stored *outside* the encrypted database.
- **Reason:** The DEK encrypts the SQLCipher DB; the manifest must be readable
  *before* unlock (it holds the wrapped DEK and Argon2id parameters needed to
  derive the KEK). Storing wrapped keys (not the DEK) in an unencrypted
  manifest is standard for this pattern and leaks no secret.
- **Alternatives:** A single encrypted blob header (more complex; SQLCipher
  cannot host its own key wrapper).
- **Security consequences:** The manifest contains no plaintext DEK or
  passphrase (verified by test); it is integrity-protected only by AES-GCM on
  the wrapped blobs. A future hardening step could sign the manifest.
- **Data consequences:** The directory layout is part of the on-disk contract;
  backups (Phase 9) package these files.
- **Reversibility:** Medium.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/vault.rs`.

## D010 — SQLCipher keyed by raw DEK hex literal (Phase 1)
- **Decision:** SQLCipher is keyed with `PRAGMA key = "x'<64 hex>'"` using the
  32-byte DEK directly (not a passphrase-derived key via SQLCipher's own KDF).
- **Reason:** Sammy owns the KDF (Argon2id, calibrated, with the chosen
  parameters) and wraps the DEK itself. Disabling SQLCipher's internal KDF by
  supplying a raw key avoids double-KDF and keeps all key derivation in our
  audited `crypto` module. (We pass the raw DEK; SQLCipher recognizes a 64-hex
  literal as a raw key and skips its KDF.)
- **Alternatives:** Let SQLCipher derive from the passphrase (loses Argon2id
  control and the separate recovery path).
- **Security consequences:** Positive — single, audited KDF; enables the
  recovery-unlock path (recovery KEK also unwraps the same DEK).
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/vault.rs`.

## D011 — One unlocked vault per process (Phase 1)
- **Decision:** `AppState` holds at most one unlocked `Vault` session.
- **Reason:** A single-user desktop app with one window benefits from a single
  active vault: it makes the "lock clears active sensitive state" guarantee
  (§10) trivial and avoids accidental cross-vault context bleed in the UI.
  Other vaults remain on disk and can be selected after locking the current one.
- **Alternatives:** Multiple simultaneously-unlocked vaults (higher cognitive
  load; more leakage surface).
- **Security consequences:** Positive — smaller in-memory secret surface.
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/app.rs`.

## D012 — Forward-only migrations with per-migration transactions (Phase 1)
- **Decision:** Migrations are forward-only, contiguous, versioned, and each
  runs inside its own SQL transaction. `schema_version` advances only after a
  migration commits; a failed migration rolls back and the version does not
  advance. There is no automatic downgrade.
- **Reason:** Directive §37 forbids "database migration without rollback
  protection." Per-migration transactions guarantee a migration is atomic; the
  version-only-after-commit rule guarantees a crash never records an un-applied
  schema. No downgrade path means we never silently destroy data to roll back.
- **Alternatives:** Out-of-order migrations (sqitch-style); automatic downgrade
  scripts (complex, error-prone, risky for an encrypted store).
- **Security consequences:** Positive — the database is never left in a
  half-migrated state.
- **Data consequences:** A failed migration is a hard error the user must
  resolve (e.g. by restoring a backup); the DB remains usable at its last good
  version.
- **Reversibility:** Low for the design (intentionally); high for any single
  migration (it rolls back).
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/db.rs`.

## D013 — Append-only audit with no mutation path (Phase 1)
- **Decision:** The `audit` module exposes only `record` and `list`/`count`
  queries. There is deliberately no `delete` or `update` for audit rows.
- **Reason:** Directive §7 calls for append-only audit history; exposing no
  mutation path makes tampering structurally harder (within the threat model —
  an attacker with the unlocked DB can still write SQL, which a future
  hardening step may address with a tamper-evident chain).
- **Alternatives:** Allow deletion with a separate privilege (rejected: violates
  append-only).
- **Security consequences:** Positive — history cannot be rewritten through the
  API.
- **Data consequences:** Audit history grows monotonically; the Privacy & Audit
  view paginates via `limit`.
- **Reversibility:** High (a future version could add redaction with approval).
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/audit.rs`.

## D014 — Native-thread inactivity checker; clock abstracted (Phase 1)
- **Decision:** The inactivity checker is a plain `std::thread` that sleeps 30s
  between checks and calls `AppState::check_inactivity_lock`. The watchdog's
  time source is abstracted behind a `Clock` trait (`SystemClock` in
  production) so the logic is unit-testable with deterministic time.
- **Reason:** Avoids pulling tokio as a direct dependency just for one timer;
  keeps the watchdog logic pure and testable. 30s granularity is well below the
  5-minute minimum policy.
- **Alternatives:** `tauri::async_runtime` timer (API not available); a tokio
  interval (extra dependency).
- **Security consequences:** Neutral — locks promptly after the threshold.
- **Data consequences:** None.
- **Reversibility:** High.
- **Date:** 2026-08-05.
- **Affected:** `src-tauri/src/lock.rs`, `src-tauri/src/lib.rs`.


