# Sammy — State

This file describes **only what currently works**, with evidence. It is updated
every checkpoint. If something is not here, it does not work yet.

Last updated: 2026-08-05 (Phase 1 in progress)

## Works now

### Foundation (Phase 0 — complete and verified)
- **Workspace builds.** `cargo build --workspace` succeeds on Windows 11 x64
  with the MSVC toolchain. SQLCipher + OpenSSL are bundled and compiled from
  source via `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature.
- **Frontend builds.** `npx vite build` produces a production bundle
  (~148 KB JS / ~2 KB CSS) in `dist/`.
- **Rust tests pass.** See Phase 1 numbers below.
- **Frontend tests pass.** `npx vitest run` → 3 passed, 0 failed.
- **Quality gate green.** `cargo fmt --check`, `cargo clippy -D warnings`,
  `eslint --max-warnings=0`, `prettier --check`, `tsc --noEmit` all pass.
- **App runs.** Tauri binary compiles with `ping`, `app_meta`, and the vault
  commands registered; React shell renders sidebar + 8 views.
- **CLI binary.** `sammy-cli --version` / `--help` work.

### Cryptographic vault spine (Phase 1 — core complete, integration ongoing)
- **`crypto` module (done).** `SecretKey` (zeroizing, constant-time eq), CSPRNG
  (ChaCha20Rng seeded from OS RNG), Argon2id KEK derivation, AES-256-GCM
  encrypt/decrypt, key wrap/unwrap, `VaultKeyMaterial` (versioned record with
  passphrase + recovery unwrap, never stores plaintext DEK), high-entropy
  recovery codes (160 bits, base32 + checksum, human-transmissible). 29 unit
  tests cover round-trips, tamper detection, wrong-key failure, and proof that
  material contains no plaintext DEK/passphrase.
- **`vault` module (done).** `VaultRegistry` over a per-user app-data dir;
  per-vault directory with `vault.json` (manifest + wrapped keys, no DEK) and
  `vault.db` (SQLCipher, keyed by the DEK). Create / unlock (passphrase or
  recovery) / lock / list / delete; multi-vault isolation enforced;
  SQLCipher DB verified not plaintext on disk; manifest verified free of
  plaintext secrets. 11 unit tests + 4 integration tests.
- **Tauri commands wired (done).** `vault_create`, `vault_list`,
  `vault_unlock`, `vault_unlock_recovery`, `vault_lock`, `vault_status`.
  `AppState` holds the registry and at most one unlocked `Vault` session.
  `AppError` serializes to a sanitized `{kind, message}` for the frontend.
- **DB baseline (done).** New vaults get an initialized SQLCipher DB with an
  `audit_events` table and a `schema_version` row.

## Verified Phase 1 exit criteria (so far)
- ✅ Private access impossible before unlock (commands check `AppState`).
- ✅ Wrong passwords fail safely (`AppError::Crypto`, opaque).
- ✅ Recovery-key restore works (passphrase OR recovery both unwrap the DEK).
- ✅ Database content is not readable as plaintext (SQLCipher; tested).
- ✅ Vaults cannot access one another (separate DEKs/DBs; tested).

## Known limitations / mocks in use
- Inactivity / Windows session-lock auto-lock not yet wired (§10).
- Versioned migration *runner* with rollback protection not yet implemented
  (baseline schema is applied directly; `schema_version` row is set).
- Audit-event *foundation* table exists but no `audit` module API records
  events yet.
- Initial full encrypted backup / restore not yet implemented.
- The eight UI views are still placeholders; vault commands are not yet
  called from the React frontend.

## External blockers
See `EXTERNAL_BLOCKERS.md`: build PATH must include Strawberry Perl (present);
no code-signing cert; no Venice API key; no KoboldCpp endpoint; OCR binding not
finalized; license report not yet generated. None block the build or tests.

## Next work
1. Versioned migration runner with rollback protection + an `audit` module API.
2. Inactivity (default 15 min) + Windows session-lock handling.
3. Initial full encrypted backup + restore.
4. Wire the React `Vaults` view to the vault commands end-to-end.
