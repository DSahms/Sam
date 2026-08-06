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
- **Fallback:** The Venice adapter interface is implemented; integration tests
  use the deterministic mock provider. Cloud-provider chat is exercised
  end-to-end when a key is supplied.

### KoboldCpp local endpoint
- **Blocker:** No KoboldCpp-compatible service running.
- **Fallback:** The KoboldCpp adapter interface is implemented; tests use the
  mock provider. Local chat is exercised end-to-end when an endpoint is
  configured.

## OCR

### Local OCR engine binding
- **Blocker:** The chosen local OCR adapter dependency has not been finalized.
- **Fallback:** Per directive §23, the OCR adapter interface and deterministic
  fixtures/mocks are implemented; scanned-page OCR is exercised against those
  until the real binding is selected.

## Dependency license report

- **Status:** Not yet generated. Directive §3 requires a dependency + license
  report compatible with future commercial distribution. SQLCipher is under a
  permissive license; OpenSSL under Apache-2.0; the Rust crates are mostly
  MIT/Apache-2.0. A machine-generated report (`cargo about` or equivalent) will
  be produced before the Phase 9 release gate.

## npm audit advisories

- **Status:** `npm install` reports advisories in dev-only transitive
  dependencies. These do not ship in the production Tauri bundle (only the
  built frontend assets are bundled). They will be addressed by dependency
  updates before the release gate.
