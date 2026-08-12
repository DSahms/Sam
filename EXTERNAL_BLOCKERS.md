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
- **Blocker:** No code-signing credential is available.
- **Fallback:** Build and test **unsigned** development packages (MSI/NSIS) per
  directive §3. Switch to a signed installer when credentials become available.

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

- **Blocker:** Only the current Windows 11 machine was available. NSIS install,
  launch, exit, and relaunch pass here; MSI creation passes, while MSI install
  requires administrator elevation. Repeat on a separate clean VM before public
  distribution.
