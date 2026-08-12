# Windows release process

[Build](build.md) · [Testing](testing.md) · [External prerequisites](../../EXTERNAL_BLOCKERS.md)

## Prerequisites

Use the development toolchain in [setup](setup.md). Confirm the version matches
`package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Supply a
code-signing certificate only through the approved release environment; never
commit it.

## Release checklist

- [ ] Working tree reviewed; no vaults, credentials, model files, or caches tracked.
- [ ] `STATE.md`, `ROADMAP.md`, `CHANGELOG.md`, and documentation match behavior.
- [ ] Full automated gate in [testing](testing.md) passes.
- [ ] `npm run tauri:build` produces release executable, MSI, and NSIS.
- [ ] Install NSIS as a current user; install MSI in an administrator session.
- [ ] Launch; create/unlock a disposable vault; open every navigation view.
- [ ] Test configured KoboldCpp and send a normal Chat message.
- [ ] Import representative TXT/PDF/DOCX/image fixtures as dependencies permit.
- [ ] Create, preview, and restore a disposable encrypted backup.
- [ ] Exit/relaunch and verify vault state is not corrupted.
- [ ] Run on an independent clean Windows 11 machine.
- [ ] Sign and verify packages when a certificate is available.

Artifacts are `target/release/bundle/msi/Sammy_<version>_x64_en-US.msi` and
`target/release/bundle/nsis/Sammy_<version>_x64-setup.exe`. Generated bundles are
not committed.
