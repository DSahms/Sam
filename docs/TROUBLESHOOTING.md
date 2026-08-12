# Troubleshooting

[Documentation home](README.md) · [Diagnostics](operations/logs-and-diagnostics.md)

Record what you were doing, the exact sanitized error, Sammy's version, and
whether the vault was locked. Never share credentials, vault files, backups, or
private prompts. Each section gives a likely cause, diagnosis, and safe fix.

## Sammy will not start or install

Confirm Windows 11 x64 and the correct installer architecture. MSI installation
requires administrator privileges; use NSIS for the current-user path. Unsigned
builds can trigger reputation controls—verify provenance rather than disabling
security globally. For source builds, verify MSVC tools, Node 22, Rust, and
Strawberry Perl.

**Diagnose:** MSI exit code `1603` with error `1925` means administrator
privileges are missing. If no window opens, check Task Manager for a Sammy process.

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
