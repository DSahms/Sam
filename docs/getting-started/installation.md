# Install Sammy on Windows

[Documentation home](../README.md) · [Quick start](quick-start.md) · [Troubleshooting](../TROUBLESHOOTING.md)

Sammy `0.1.0` supports 64-bit Windows 11. You do not need Rust, Node.js, Git, or
programming tools when installing a prebuilt package.

Related: [Windows security warnings](windows-security-warnings.md) ·
[Verify hashes and signatures](verify-windows-packages.md) ·
[Clean-Windows validation](windows-clean-install-validation.md)

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
3. Review any Windows publisher warning. Current development builds are unsigned.
   Check the SHA-256 first ([verify packages](verify-windows-packages.md)). See
   [Windows security warnings](windows-security-warnings.md) for what SmartScreen
   and UAC mean. Do not turn those protections off.
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

## Uninstall, residue, and reinstall

Use **Settings → Apps → Installed apps**, find Sammy, and choose **Uninstall**.

What that removes:

- Program files (`%LOCALAPPDATA%\Sammy` for NSIS, or `C:\Program Files\Sammy` for MSI)
- The Start Menu shortcut named Sammy
- The uninstall registry entry for that installer

What that does **not** remove by default:

- Vaults and settings under `%LOCALAPPDATA%\app.sammy.desktop`

The NSIS uninstaller can offer **Delete app data**. Leave it unchecked unless you
intentionally want every local vault gone and already have a tested encrypted
backup. Uninstalling the program is not a backup.

### Reinstall

Install the same version again after uninstall. The vault list should reappear if
`app.sammy.desktop` was left in place. If you checked delete-app-data, you are
starting from an empty profile.

### Why two folders

`%LOCALAPPDATA%\Sammy` is “the program.” `%LOCALAPPDATA%\app.sammy.desktop` is
“your data.” Mixing them would make an uninstall look like a wipe.

Before uninstalling or moving computers, create and test an encrypted backup.
Do not manually delete the application-data folder unless you intentionally want
to remove every local vault and already have a verified recovery path.

Public distribution still needs a signed installer and a
[clean-Windows validation](windows-clean-install-validation.md) pass on a
machine that is not the development PC. Both remain outstanding.
