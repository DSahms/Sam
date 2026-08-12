# Development setup

[Documentation home](../README.md) · [Build](build.md) · [Testing](testing.md)

Use Windows 11 x64 with Git, Node.js 22/npm, Rust stable with the MSVC target,
Visual Studio C++ build tools and Windows SDK, and Strawberry Perl. Vendored
OpenSSL requires Perl modules missing from Git for Windows' minimal Perl; place
`C:\Strawberry\perl\bin` and `C:\Strawberry\c\bin` before it on `PATH`.

```powershell
git clone <repository-url>
Set-Location Sammy
npm install
cargo build --workspace
npm run tauri:dev
```

Development vaults default to `%LOCALAPPDATA%\app.sammy.desktop\vaults`; do not
commit them. The Tauri dev server uses `http://localhost:1420`. No Venice key is
required for deterministic tests.

The legacy [development runbook](../DEVELOPMENT_RUNBOOK.md) contains additional
toolchain notes.
