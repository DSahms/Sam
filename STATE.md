# Sammy — State

This file describes **only what currently works**, with evidence. It is updated
every checkpoint. If something is not here, it does not work yet.

Last updated: 2026-08-05 (Phase 1 in progress — vault spine + UI functional)

## Works now

### Foundation (Phase 0 — complete and verified)
- **Workspace builds.** `cargo build --workspace` succeeds on Windows 11 x64
  (MSVC toolchain). SQLCipher + OpenSSL bundled via
  `rusqlite`'s `bundled-sqlcipher-vendored-openssl`.
- **Frontend builds.** `npx vite build` produces a production bundle (~154 KB JS
  / ~4 KB CSS).
- **Quality gate green end-to-end.** `cargo fmt --check`,
  `cargo clippy -D warnings`, `eslint --max-warnings=0`, `prettier --check`,
  `tsc --noEmit`, `cargo test`, `vitest` all pass.
- **App runs.** Tauri binary registers all commands; React shell renders the
  sidebar + 8 views; the Vaults view is fully functional.
- **CLI binary.** `sammy-cli --version` / `--help` work.

### Cryptographic vault spine (Phase 1 — core complete)
- **`crypto` module.** `SecretKey` (zeroizing, constant-time eq), CSPRNG,
  Argon2id KEK derivation, AES-256-GCM, key wrap/unwrap, `VaultKeyMaterial`
  (versioned, passphrase + recovery unwrap), high-entropy recovery codes
  (160-bit, base32+checksum). Verified: no plaintext DEK in material.
- **`vault` module.** Per-vault directory (`vault.json` + `vault.db`); create /
  unlock (passphrase or recovery) / lock / list / delete; multi-vault isolation;
  SQLCipher encrypted-at-rest (tested); manifest free of plaintext secrets.
- **`db` module.** Versioned, forward-only migrations with transactional
  rollback protection (directive §37). v2–v4 define knowledge_records,
  conversations/messages, sources tables. Applied automatically on unlock.
- **`audit` module.** Append-only `audit_events` API: record / list /
  count_by_category. No delete/update path. 9 categories.
- **`lock` module.** `LockPolicy` (5/15/30/60 min, manual-only; default 15),
  `InactivityWatchdog` (touch / should_lock), `SystemClock`. Integrated into
  `AppState`; a native background thread checks every 30s and locks on
  inactivity.
- **Tauri commands.** `vault_create/list/unlock/unlock_unlock_recovery/lock/
  status`, `touch_activity`, `lock_policy_get/set`. `AppError` serializes to a
  sanitized `{kind, message}`.
- **Frontend Vaults view (functional).** The owner can create a vault (with a
  one-time recovery-code display), unlock by passphrase or recovery code, lock,
  enumerate vaults, and pick the inactivity policy — all against real
  SQLCipher-backed storage.

## Verified Phase 1 exit criteria
- ✅ Private access impossible before unlock (commands check `AppState`).
- ✅ Wrong passwords fail safely (opaque `AppError::Crypto`).
- ✅ Recovery-key restore works (passphrase OR recovery both unwrap the DEK).
- ✅ Database content not readable as plaintext (SQLCipher; integration-tested).
- ✅ Vaults cannot access one another (separate DEKs/DBs; integration-tested).
- ✅ Locking clears active sensitive state (the `Vault` session is dropped).
- ✅ DB migration has rollback protection (broken-migration rollback test).

## Not yet done in Phase 1
- Inactivity lock works, but Windows **session-lock** signal is not yet hooked
  (the periodic inactivity checker covers the common case; OS session-lock
  detection is the next hardening step).
- Initial full encrypted backup / restore (§30/§31) not yet implemented — this
  is the next major slice.
- App-exit auto-lock relies on process termination dropping the session; an
  explicit lock-on-exit hook is not yet wired.
- Audit events are recordable but no command currently records them (the API is
  ready; callers in Phase 2+ will use it).

## Known limitations / mocks in use
- Phase 2+ subsystems (identity, conversation, providers, knowledge, corpus,
  sources, retrieval, citations, memory, permissions, tools, backup) are stubs.
- Chat / What I Know / Sources / Memory / Privacy / Backup / Settings views are
  placeholders.
- No code-signing; builds are unsigned dev packages.

## External blockers
See `EXTERNAL_BLOCKERS.md`: build PATH must include Strawberry Perl (present);
no code-signing cert; no Venice API key; no KoboldCpp endpoint; OCR binding not
finalized; license report not yet generated. None block the build or tests.

## Next work
1. Initial full encrypted backup + restore (Phase 1's last major slice).
2. Hook the Windows session-lock signal to lock the vault.
3. Begin Phase 2 (identity, conversation, mock/KoboldCpp/Venice providers).
