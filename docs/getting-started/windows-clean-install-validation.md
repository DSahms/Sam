# Clean-Windows installer validation

[Installation](installation.md) · [Verify packages](verify-windows-packages.md) · [Code signing](../development/windows-code-signing.md)

This page is the acceptance matrix for proving Sammy can be installed, used,
uninstalled, and reinstalled on a Windows computer that does **not** depend on
the development machine.

Quick procedure first. Why it matters afterward.

## What “clean Windows” means

A clean-Windows check answers one question: **would a person who never cloned
this repository be able to install Sammy and use it?**

That machine must not rely on:

- the Sammy source tree
- Rust, Node.js, npm, or a developer PATH
- files under `target\release` except the installer you copied in
- `tools\pkc-clickthrough-fixture` or any other fixture script
- development-only environment variables

If a check was done on the development PC, it is useful evidence. It is **not**
independent clean-Windows validation.

## Status on 2026-08-17

| Check | Result |
| --- | --- |
| Independent clean VM / Windows Sandbox / spare PC | **UNVALIDATED** — none of those targets exist on this host |
| Host forensics of the produced installers | Done on Windows 11 Pro 10.0.26200 |
| MSI administrative extract (no Program Files install) | Done |
| NSIS payload extract + isolated `LOCALAPPDATA` launch | Done |
| Live NSIS/MSI install on this development PC | **Not performed** — this PC already has a daily Sammy install under `%LOCALAPPDATA%\Sammy` that must not be overwritten |

Do not treat the host checks as a PASS for public distribution.

## Quick procedure (human, on a spare Windows 11 PC)

Copy **only** the installer and the SHA-256 value from the owner. Do not copy
the repository.

1. Confirm **Settings → System → About** shows Windows 11, 64-bit.
2. Confirm Rust, Node, Git, and the Sammy repo are **not** required. If they are
   already installed, that is allowed, but do not use them during the test.
3. Verify the installer SHA-256 before running it. See
   [Verify Windows packages](verify-windows-packages.md).
4. Run the chosen installer (NSIS first unless you are specifically testing MSI).
5. Launch Sammy from the Start Menu.
6. Create a disposable vault, save the recovery code off-device, unlock, lock,
   unlock again, then quit and relaunch.
7. In **Settings**, leave personal knowledge (PKC) **off**. Configure KoboldCpp
   only if a local model service exists on *that* machine.
8. Uninstall from **Settings → Apps**. Inspect residue. Reinstall. Repeat the
   first-launch vault check.
9. Record every row in the matrix below as PASS, FAIL, or UNVALIDATED.

Stop and keep the machine’s security settings intact if SmartScreen or Defender
blocks the file. Do not disable those controls to force a green result.

## Acceptance matrix

Mark each row with the environment used. `Host` means the development PC.
`Isolated payload` means extracted installer files launched with a temporary
`LOCALAPPDATA`. `Clean PC` means a machine or VM that never had the Sammy
source tree.

| # | Check | What success looks like | Host 2026-08-17 | Clean PC |
| --- | --- | --- | --- | --- |
| 1 | MSI install | Completes to `C:\Program Files\Sammy` after a UAC prompt | UNVALIDATED (admin extract only; no per-machine install) | UNVALIDATED |
| 2 | NSIS install | Completes for the current user to `%LOCALAPPDATA%\Sammy` without requiring admin | UNVALIDATED (would overwrite the daily install) | UNVALIDATED |
| 3 | First launch | Window title `Sammy`; Vaults page; no source-tree path required | PASS — extracted NSIS `sammy.exe` launched with isolated `%LOCALAPPDATA%` | UNVALIDATED |
| 4 | Vault creation | New vault appears; creation does not auto-unlock | UNVALIDATED on this isolated payload (window only) | UNVALIDATED |
| 5 | Recovery-code display | One-time code shown; owner must confirm it was saved | UNVALIDATED here; covered by packaged PKC click-through on the host exe | UNVALIDATED |
| 6 | Lock / unlock | Correct passphrase unlocks; lock returns to the locked Vaults state | UNVALIDATED here; host packaged click-through previously passed | UNVALIDATED |
| 7 | Application restart | Quit and relaunch; vault list still present | UNVALIDATED here; isolated profile created `app.sammy.desktop\vaults` | UNVALIDATED |
| 8 | Settings persistence | Saved provider/PKC settings survive restart | UNVALIDATED here; host packaged click-through previously passed | UNVALIDATED |
| 9 | KoboldCpp configuration | Settings accept a user-entered endpoint; default is empty until saved | Config is user-editable; not a hidden install dependency | UNVALIDATED |
| 10 | PKC disabled behavior | Default is off; chat still works without personal-knowledge provenance | Default-off is product behavior; isolated first launch did not enable it | UNVALIDATED |
| 11 | Uninstall | Apps & features removes the program files and Start Menu shortcut | UNVALIDATED (daily install left untouched) | UNVALIDATED |
| 12 | Residue inspection | `%LOCALAPPDATA%\app.sammy.desktop` remains unless the owner opts to delete app data | Scripted NSIS uninstaller has an **unchecked** “delete app data” box | UNVALIDATED |
| 13 | Reinstall | Same-version installer can run again and launch | UNVALIDATED | UNVALIDATED |
| 14 | Upgrade-readiness | A *newer version* replaces the old one without inventing a fake release | UNVALIDATED — product is still `0.1.0`; do not fake a version bump | UNVALIDATED |

## What this development PC can prove vs what it cannot

### Can prove here (and was proved)

- The MSI and NSIS files exist, have stable names, and hash to known SHA-256
  values before any rebuild.
- Both installers contain `sammy.exe` and `sammy-cli.exe`.
- MSI is WiX per-machine (`Program Files\Sammy`). NSIS is per-user
  (`%LOCALAPPDATA%\Sammy`).
- Neither installer is Authenticode-signed.
- An MSI administrative extract (`msiexec /a ... TARGETDIR=...`) succeeds
  without installing into Program Files.
- The NSIS payload, extracted with 7-Zip, launches when `LOCALAPPDATA` is
  redirected to a temp folder. It created `%TEMP%\...\app.sammy.desktop\vaults`.
- The application executable depends on Windows system / Universal CRT DLLs
  plus the already-installed WebView2 runtime. SQLCipher and OpenSSL are
  statically bundled. Tesseract is **not** shipped.
- A real portability defect was found and fixed: Settings **Find local defaults**
  used to compile `F:\personal-knowledge-corpus-scaffold\...` and
  `D:\dev\StoryKeeper\...` into `sammy.exe`. Those strings are gone from
  production discovery. The owner still chooses PKC folders; optional env
  `SAMMY_PKC_ROOT` / `SAMMY_PKC_BRIDGE` can hint discovery without baking paths.

### Cannot prove here without being dishonest

- A person with no Sammy repo, no Rust, and no prior install can complete
  MSI or NSIS setup.
- SmartScreen / Defender behavior on a machine that has never seen these
  files.
- Uninstall and residue after a real Apps & features removal.
- Upgrade from version N to N+1.

Windows Sandbox is not installed on this host. Hyper-V’s service is running,
but no virtual machines exist. This milestone does not create VMs or change
Windows features.

## Installer facts used by the matrix

These come from `src-tauri/tauri.conf.json` and the generated WiX/NSIS scripts.
The configuration was **not** changed for this milestone.

| Property | NSIS | MSI |
| --- | --- | --- |
| Filename | `Sammy_0.1.0_x64-setup.exe` | `Sammy_0.1.0_x64_en-US.msi` |
| Technology | NSIS 3 Unicode | WiX v3 / Windows Installer |
| Architecture | x64 | x64 |
| Product version | 0.1.0 | 0.1.0 |
| Install scope | Current user (`RequestExecutionLevel user`) | Per-machine (`ALLUSERS=1`) |
| Default location | `%LOCALAPPDATA%\Sammy` | `C:\Program Files\Sammy` |
| Start Menu | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Sammy.lnk` | Start Menu shortcut |
| Desktop shortcut | Optional on the finish page; silent/passive install creates one | Created by the MSI |
| Uninstall registration | `HKCU\...\Uninstall\Sammy` | ARP / Windows Installer product |
| Upgrade code | NSIS product name `Sammy` | `{1A32F789-C027-5F2A-BAC9-FDFBBC946EA4}` (stable) |
| App data (vaults) | `%LOCALAPPDATA%\app.sammy.desktop` | Same |
| WebView2 if missing | Download bootstrapper from Microsoft | Same CustomAction |
| Elevation | None for a normal per-user install | Administrator UAC |

Runtime data is intentionally **not** stored next to `sammy.exe`. Uninstalling
the program is not the same as deleting vaults.

## Isolated-payload procedure used on the development PC

This is the strongest non-destructive check available without touching the
daily install.

```powershell
# 1. Administrative extract of the MSI (does not install to Program Files)
msiexec /a "target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi" /qn `
  TARGETDIR="$env:TEMP\sammy-msi-admin-extract"

# 2. Extract the NSIS payload (inspection only)
& "C:\Program Files\7-Zip\7z.exe" x -y `
  "-o$env:TEMP\sammy-nsis-extract" `
  "target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe"

# 3. Launch the extracted EXE with a throwaway profile
$profile = "$env:TEMP\sammy-payload-isolation"
New-Item -ItemType Directory -Force -Path $profile | Out-Null
$env:LOCALAPPDATA = $profile
Start-Process "$env:TEMP\sammy-nsis-extract\sammy.exe"
```

Success: a window titled **Sammy** appears, and
`$profile\app.sammy.desktop\vaults` is created. Stop the process when finished.
Do not point `LOCALAPPDATA` at your real profile while doing this.

## Why the inner `sammy.exe` hash may differ from `target\release\sammy.exe`

The standalone release executable, the copy inside the MSI, and the copy inside
the NSIS payload can differ by a few bytes (observed: 3 bytes) after packaging.
Record and publish the SHA-256 of **the installer files the user actually
receives**, not only the unpackaged `sammy.exe`.

## Why this matrix exists

Installers can silently depend on the builder’s machine: extra DLLs, absolute
paths, fixture folders, or “it works because WebView2 / MSVC / Node is already
there.” A clean PC is the only honest test of those assumptions. Until that
machine exists, the remaining rows stay **UNVALIDATED**.
