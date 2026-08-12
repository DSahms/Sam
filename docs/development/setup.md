# Build Sammy from source

[Documentation home](../README.md) · [Build](build.md) · [Testing](testing.md)

This path is for developers. If you only want to use Sammy, install a prebuilt
package using the [Windows installation guide](../getting-started/installation.md).

## Supported environment

The first production target is **Windows 11 x64**. Run the commands below in
**PowerShell**, Microsoft's command shell. To open it, open the Start menu, type
`PowerShell`, and select **Windows PowerShell**. Administrator mode is not needed
for normal repository commands, but installing system prerequisites may request it.

## Install and verify prerequisites

| Software | Why Sammy needs it | Verification command |
| --- | --- | --- |
| Git | Downloads and versions the source repository | `git --version` |
| Node.js 22 and npm | Builds the React/TypeScript interface | `node --version` and `npm --version` |
| Rust stable with MSVC | Builds the Rust/Tauri backend | `rustc --version` and `cargo --version` |
| Visual Studio C++ Build Tools + Windows SDK | Supplies the Windows compiler/linker | Open **Visual Studio Installer** and confirm **Desktop development with C++** |
| Strawberry Perl | Configures vendored OpenSSL/SQLCipher | `C:\Strawberry\perl\bin\perl.exe --version` |

Install Git, Node.js, and Visual Studio Build Tools from their official vendor
installers; install Rust through `rustup`; install Strawberry Perl from its
official Windows distribution. Close and reopen PowerShell after installation so
new commands are added to `PATH`—the list of folders Windows searches for programs.

Node should report major version `22`. Rust and Cargo should both print versions
without “command not found.” Strawberry Perl must be preferred over the limited
Perl bundled with Git. For the current PowerShell window:

```powershell
$env:Path = "C:\Strawberry\perl\bin;C:\Strawberry\c\bin;$env:Path"
perl --version
```

This changes `PATH` only for that open PowerShell session. It does not modify a
permanent system setting.

## Obtain the source

Choose a parent folder where you keep projects. In PowerShell, replace
`<repository-url>` with the real Git URL supplied by the project owner:

```powershell
Set-Location C:\Users\YourName\Documents
git clone <repository-url>
Set-Location Sammy
```

`Set-Location Sammy` is important: every following command must run from the
repository root, the folder containing `package.json`, `Cargo.toml`, and `README.md`.
If you downloaded a ZIP instead, extract it and use `Set-Location` with that folder.

## Install dependencies and launch

```powershell
npm install
cargo build --workspace
npm run tauri:dev
```

`npm install` downloads the exact frontend packages recorded in
`package-lock.json`. `cargo build` downloads and compiles Rust dependencies.
`tauri:dev` starts Vite at `http://localhost:1420`, builds the backend, and opens
the desktop application.

Success looks like a **Sammy** window opening on **Vaults**. Stop the development
process by returning to PowerShell and pressing `Ctrl+C` after closing the app.

Development vaults default to `%LOCALAPPDATA%\app.sammy.desktop\vaults`; never
commit them. A Venice key is not required for builds or deterministic tests.

Next: [run the tests](testing.md) · [understand the repository](repository-layout.md) · [create installers](release-process.md)
