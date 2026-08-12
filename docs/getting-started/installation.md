# Install Sammy on Windows

[Documentation home](../README.md) · [Quick start](quick-start.md) · [Troubleshooting](../TROUBLESHOOTING.md)

Sammy `0.1.0` supports 64-bit Windows 11. You do not need Rust, Node.js, Git, or
programming tools when installing a prebuilt package.

## Before you begin

You need:

- A Windows 11 computer with an x64 processor.
- A Sammy installer obtained from the project owner or a trusted release location.
- Permission to run an installer. The NSIS option installs only for your current
  Windows account; MSI installation requires an administrator.
- A local model service such as KoboldCpp if you want real AI responses without
  using an optional cloud provider. Sammy itself does not contain a language model.

To confirm your Windows version, open **Settings → System → About** and look for
**Windows specifications** and **System type**. It should say Windows 11 and a
64-bit operating system.

## Choose an installer

| Format | Filename | Choose this when |
| --- | --- | --- |
| NSIS setup | `Sammy_0.1.0_x64-setup.exe` | You want the simplest current-user installation and do not have administrator access |
| Windows Installer | `Sammy_0.1.0_x64_en-US.msi` | An administrator or managed deployment workflow requires MSI |

The filenames above are build outputs, not public download links. If you received
only the source repository, follow [build from source](../development/setup.md) to
create them beneath `target\release\bundle`.

## Install with NSIS (recommended)

1. In File Explorer, open the folder containing `Sammy_0.1.0_x64-setup.exe`.
2. Double-click the file.
3. Review any Windows publisher warning. Current development builds are unsigned,
   so verify that the file came from the expected source before continuing.
4. Complete the setup. It installs Sammy for the signed-in Windows user beneath
   `%LOCALAPPDATA%\Sammy`.
5. Launch **Sammy** from its installed application entry.

Success looks like a window titled **Sammy** with a left navigation bar and the
**Vaults** page open. Continue with [First run](first-run.md).

## Install with MSI

1. Sign in to an account that can approve administrator installation.
2. Double-click `Sammy_0.1.0_x64_en-US.msi` and approve the elevation request.
3. Complete the Windows Installer steps and launch Sammy.

An MSI error `1925` or exit code `1603` commonly means the installation was not
started with the required administrator privileges. Use the NSIS installer if a
per-user install is sufficient.

> [!CAUTION]
> Do not disable Smart App Control, Microsoft Defender, or reputation protection
> globally to run an unsigned build. Verify the source, or wait for a signed build.

## Uninstall and retain data safely

Use **Settings → Apps → Installed apps**, find Sammy, and choose **Uninstall**.
Application data is deliberately separate under
`%LOCALAPPDATA%\app.sammy.desktop`; uninstalling the executable must not be
treated as a data-deletion or backup operation.

Before uninstalling or moving computers, create and test an encrypted backup.
Do not manually delete the application-data folder unless you intentionally want
to remove every local vault and already have a verified recovery path.
