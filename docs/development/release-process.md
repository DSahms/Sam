# Windows release process

[Build](build.md) · [Testing](testing.md) · [Code signing](windows-code-signing.md) · [External prerequisites](../../EXTERNAL_BLOCKERS.md)

## Prerequisites

Use the development toolchain in [setup](setup.md). Confirm the version matches
`package.json`, workspace `Cargo.toml`, and `src-tauri/tauri.conf.json`. Supply a
code-signing certificate only through the approved release environment; never
commit it. Read [Windows code signing](windows-code-signing.md) before adding
any `bundle.windows` keys.

## Release order (what “done” looks like)

1. Clean source — no vaults, credentials, model files, caches, or `.signing/` files tracked.
2. Automated tests and quality gates in [testing](testing.md).
3. Production build: `npm run tauri:build` from the repository root.
4. Sign `sammy.exe` (blocked until a certificate exists).
5. Sign or regenerate MSI and NSIS as the chosen Tauri signing mode requires.
6. Verify signatures, timestamps, and certificate chain.
7. Record SHA-256 for the files users actually receive:
   - `target\release\bundle\msi\Sammy_<version>_x64_en-US.msi`
   - `target\release\bundle\nsis\Sammy_<version>_x64-setup.exe`
   - and, if published separately, `target\release\sammy.exe`
8. Run [clean-Windows validation](../getting-started/windows-clean-install-validation.md)
   on a machine that does not have the Sammy source tree.
9. Publish only after step 8 is a real PASS, not a host-machine substitute.

Today steps 4–6 and 8 are blocked or UNVALIDATED. Unsigned hashes from step 7
are still required so a recipient can check the file.

Inventory a local build without installing:

```powershell
powershell -File tools\windows-release\inventory-artifacts.ps1
```

## Release checklist

- [ ] Working tree reviewed; no vaults, credentials, model files, or caches tracked.
- [ ] `STATE.md`, `ROADMAP.md`, `CHANGELOG.md`, and documentation match behavior.
- [ ] Full automated gate in [testing](testing.md) passes, including
      `windows_packaging` tests.
- [ ] `npm run tauri:build` produces release executable, MSI, and NSIS under
      `target/release` (repository root), not `src-tauri/target`.
- [ ] SHA-256 recorded for MSI, NSIS, and EXE.
- [ ] Authenticode status recorded. Unsigned is expected until a certificate exists.
- [ ] On a **clean** Windows 11 PC: NSIS current-user install; MSI administrator install.
- [ ] Launch; create/unlock a disposable vault; recovery code shown; lock/unlock; restart.
- [ ] Settings persist; KoboldCpp configured from Settings; PKC left disabled unless that PC has PKC.
- [ ] Uninstall, inspect residue, reinstall.
- [ ] Sign and verify packages when a certificate is available.

Do not invent a fake version bump to “test upgrade.” Same-version reinstall is
the upgrade-readiness check until the version actually changes.

Generated bundles are not committed.
