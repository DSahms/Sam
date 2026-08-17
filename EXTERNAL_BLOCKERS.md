# External Blockers

External dependencies and credentials that are unavailable during development,
recorded per directive §6. Each blocker is paired with the fallback in use so
work continues. None of these block the build or the test suite.

## Build-time

### Build PATH must include Strawberry Perl
- **Blocker:** `bundled-sqlcipher-vendored-openssl` compiles OpenSSL from C
  source, whose `Configure` script requires Perl with `Locale::Maketext::Simple`,
  `Params::Check`, and `IPC::Cmd`. The Perl bundled with Git for Windows is
  missing these modules.
- **Status:** Strawberry Perl is installed at `C:\Strawberry\` and works. Builds
  must run with `C:\Strawberry\perl\bin` and `C:\Strawberry\c\bin` ahead in
  `PATH` (see `docs/DEVELOPMENT_RUNBOOK.md`). Not a code blocker; documented
  for CI setup.

## Code signing

### Windows code-signing certificate
- **Blocker:** No code-signing credential is available. `tauri.conf.json` has
  no `bundle.windows` signing block. EXE, MSI, and NSIS currently verify as
  `NotSigned`.
- **Fallback:** Build and test **unsigned** development packages (MSI/NSIS).
  Readiness plan: `docs/development/windows-code-signing.md`. Do not purchase
  or enroll a certificate as part of ordinary development.
- **Still required for public distribution:** a public CA Authenticode cert or
  a cloud signing service, timestamping, signatures on EXE + both installers +
  the NSIS uninstaller, and independent `signtool verify` / SHA-256 publication.

## AI providers

### Venice cloud API key
- **Blocker:** No Venice API key configured.
- **Fallback:** The production HTTPS transport, encrypted credential storage,
  endpoint restriction, consent routing, connection test, and deterministic
  integration coverage are complete. Only live-account validation awaits a key.

### KoboldCpp local endpoint
- **Status:** Available during the release pass. Model discovery and a normal
  chat response were verified live at localhost:5001.

## OCR

### Local OCR runtime
- **Blocker:** Tesseract is not installed on the validation machine.
- **Fallback:** The production adapter invokes local Tesseract without plaintext
  temporary files and returns a clear error when absent.

## Dependency license report

- **Status:** Generated. See `LICENSES.md` — 462 crates scanned from Cargo.lock.
  **No GPL/copyleft crates found.** All resolved dependencies use permissive
  licenses (MIT, Apache-2.0, BSD, MPL-2.0, Zlib, Unicode-3.0, CC0). 171 crates
  show as "unknown" because they use `license.workspace = true` in their
  manifests (mostly the `smol`/`async-ecosystem` crates, known MIT/Apache-2.0);
  a `cargo about` pass with the crates.io API can resolve these definitively
  before the final release gate.

## npm audit advisories

- **Status:** Resolved. `npm audit` reports zero vulnerabilities.

## Independent clean-machine validation

- **Blocker:** No Windows Sandbox, spare PC, or existing Hyper-V VM is available
  on the development host (`WindowsSandbox.exe` missing; `Get-VM` empty). The
  host already has a daily NSIS install under `%LOCALAPPDATA%\Sammy` that must
  not be overwritten for a smoke test.
- **What was done instead (2026-08-17):** installer forensics; MSI administrative
  extract; NSIS payload extract; launch of the extracted `sammy.exe` with an
  isolated `LOCALAPPDATA`. That is **not** independent clean-Windows validation.
- **Fallback:** Human procedure and matrix in
  `docs/getting-started/windows-clean-install-validation.md`. Remaining install /
  uninstall / residue / MSI / SmartScreen-on-a-fresh-PC rows stay **UNVALIDATED**.
