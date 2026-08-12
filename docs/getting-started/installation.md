# Installation

[Documentation home](../README.md) · [Quick start](quick-start.md) · [Troubleshooting](../TROUBLESHOOTING.md)

Sammy `0.1.0` targets 64-bit Windows 11. Release builds currently produce unsigned development packages.

| Format | File | Use when |
| --- | --- | --- |
| NSIS | `Sammy_0.1.0_x64-setup.exe` | Installing for the current Windows user |
| MSI | `Sammy_0.1.0_x64_en-US.msi` | An administrator or managed workflow requires MSI |

Builds place both beneath `target/release/bundle/`. NSIS installs the executable
beneath `%LOCALAPPDATA%\Sammy`; private data is separately stored beneath
`%LOCALAPPDATA%\app.sammy.desktop`.

> [!CAUTION]
> Packages are not code-signed yet. Verify their source. Do not disable Windows
> security controls globally to run an unsigned artifact.

Removing the application is not a backup strategy. Create and validate an
encrypted backup before uninstalling or moving machines.
