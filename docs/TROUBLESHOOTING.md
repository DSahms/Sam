# Troubleshooting

[Documentation home](README.md) · [Diagnostics](operations/logs-and-diagnostics.md)

Record what you were doing, the exact sanitized error, Sammy's version, and
whether the vault was locked. Never share credentials, vault files, backups, or
private prompts. Each section gives a likely cause, diagnosis, and safe fix.

## Sammy will not start or install

Confirm Windows 11 x64 and the correct installer architecture. MSI installation
requires administrator privileges; use NSIS for the current-user path. Unsigned
builds can trigger reputation controls—verify the SHA-256 rather than disabling
security globally. See [Windows security warnings](getting-started/windows-security-warnings.md)
and [verify packages](getting-started/verify-windows-packages.md). For source
builds, verify MSVC tools, Node 22, Rust, and Strawberry Perl.

**Diagnose:** MSI exit code `1603` with error `1925` means administrator
privileges are missing. If no window opens, check Task Manager for a Sammy process.

| What you see | Likely cause | What to do |
| --- | --- | --- |
| SmartScreen “Windows protected your PC” | Unsigned development package | Check SHA-256; do not disable SmartScreen |
| MSI UAC prompt | Per-machine install into Program Files | Expected. Approve only if you intended MSI |
| NSIS asks for administrator | Unexpected for the current-user installer | Stop and record it; use the matrix notes |
| App does not draw / blank window | WebView2 runtime missing | Allow the official Microsoft WebView2 bootstrapper |
| Installer runs but Start Menu missing | Wrong account or failed shortcut step | Reinstall; look under `%LOCALAPPDATA%\Sammy` |
| Reinstall looks empty | App data was deleted, or a different Windows user | Check `%LOCALAPPDATA%\app.sammy.desktop` |

## Vault will not unlock

Check the intended vault, keyboard layout, and complete passphrase. Errors are
deliberately generic. Use the saved recovery code plus a new passphrase when the
old one is forgotten. No recovery exists when both are lost.

**Diagnose:** confirm the vault name/ID, Caps Lock, and keyboard layout. Do not
edit `vault.json` or move files between vault directories.

## KoboldCpp fails or no model appears

Confirm it is running, has a model loaded, and exposes `/v1/models` at the saved
host/port (default `localhost:5001`). Test in Settings, then save the discovered
model. A timeout may mean loading or generation exceeded 120 seconds.

**Diagnose:** use **Test connection**. “Connection refused” means no service
answered that address; an empty model list usually means no model is loaded or
the compatible API is disabled.

## Chat uses an unexpected provider

Inspect the provider/model badge, saved enabled settings, and routing mode.
`local_only` fails closed when no usable local provider exists. Cloud paths
require the applicable consent; Mock is deterministic and non-cloud.

## Personal knowledge (PKC) will not connect

Unlock the vault, open **Settings → Personal knowledge (PKC)**, and use
**Test connection**. The badge is only green after a real check.

| What you see | Likely cause | What to do |
| --- | --- | --- |
| Off | Feature disabled (the default) | Enable only if you want local read-only lookups |
| Not tested | Paths saved, not verified | Test connection |
| Python missing | Python is not installed or the path is wrong | Install Python or set the executable under Advanced |
| Bridge missing | The bridge file moved | Choose the absolute bridge path in Settings |
| Unavailable | PKC folder missing or the process failed | Confirm the PKC location still exists |
| Unauthorized | Sammy is not allowed for that source | Do not weaken permissions; use an authorized source |
| Misconfigured | Source identity or paths incomplete | Save a source identity and valid folders, then retest |

Chat continues without personal knowledge. Cloud chats never consult PKC.
Turn the feature off in Settings to stop lookups immediately.

## Venice rejects the key

Unlock the vault, replace the key in Settings, and test. Authentication and rate
limits are reported without echoing the key or prompt. Sammy accepts credentialed
Venice traffic only at `https://api.venice.ai/api/v1`.

## Document import fails

Unsupported extensions fail explicitly. Invalid JSON/CSV/PDF/DOCX is not indexed.
Scanned PDFs without text are not rasterized automatically. Image OCR requires
`tesseract` on `PATH`; textual formats do not. Check write access to application data.

**Fix:** convert a scanned PDF page to a supported image and import it after
installing Tesseract, or use a PDF with a real text layer. Re-export malformed
DOCX/JSON/CSV rather than renaming its extension.

## Backup or restore fails

Confirm destination access and both backup credentials. Preview before restore.
Wrong credentials, corrupted/authentication-failed packages, unsupported versions,
or unsafe paths are rejected. Remove neither staging nor current vault files
manually; preserve both and diagnose from the sanitized error.

## Build fails

Run the commands in [development setup](development/setup.md). OpenSSL configure
errors commonly indicate Git’s incomplete Perl won precedence over Strawberry
Perl. A final `LNK4099` PDB warning alone is non-fatal; use command exit status.

**Diagnose:** run each prerequisite verification command separately. Confirm
PowerShell is in the repository root and `perl --version` resolves to Strawberry
Perl before deleting caches or reinstalling dependencies.
