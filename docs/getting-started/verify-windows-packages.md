# Verify Windows packages

[Installation](installation.md) · [Security warnings](windows-security-warnings.md) · [Code signing](../development/windows-code-signing.md)

Use this page to check that an installer is the file the owner intended, and —
once Sammy is signed — that Windows can name the publisher.

## Quick procedure: check the SHA-256

On the computer that received the installer, open PowerShell in that folder:

```powershell
Get-FileHash .\Sammy_0.1.0_x64-setup.exe -Algorithm SHA256
Get-FileHash .\Sammy_0.1.0_x64_en-US.msi -Algorithm SHA256
```

Compare the `Hash` line to the value the owner published for **that exact
filename**. Comparison is case-insensitive. If they differ, stop.

You can also inventory a local build from the repository root:

```powershell
powershell -File tools\windows-release\inventory-artifacts.ps1
```

That script prints size, SHA-256, and whether a signature is present. It does
not install Sammy and must never print private keys.

## Why a hash check

SHA-256 is a fingerprint of the bytes. It answers “is this the same file?” It
does not answer “is this file safe?” or “who made it?” For those you need a
signature plus a publisher you trust.

The installer you were given is the file to hash. Do not hash a copy you
extracted yourself and call that the published artifact. Packaging can change a
few bytes inside the nested `sammy.exe`; the published number is for the MSI or
NSIS file the user runs.

## Quick procedure: check for a signature

```powershell
Get-AuthenticodeSignature .\Sammy_0.1.0_x64-setup.exe | Format-List *
Get-AuthenticodeSignature .\Sammy_0.1.0_x64_en-US.msi | Format-List *
Get-AuthenticodeSignature .\sammy.exe | Format-List *
```

| Status | Meaning for Sammy today |
| --- | --- |
| `NotSigned` | Expected. Current builds are unsigned. |
| `Valid` | A signature is present and Windows accepts the chain. Read the signer name. |
| `HashMismatch` / `NotTrusted` / `UnknownError` | Do not install. Record the status. |

On a signed release you should also see a timestamp. In `signtool` form:

```powershell
signtool verify /pa /v .\Sammy_0.1.0_x64-setup.exe
signtool verify /pa /v .\Sammy_0.1.0_x64_en-US.msi
signtool verify /pa /v .\sammy.exe
```

`/pa` uses the default Authenticode policy. `/v` prints the signer, chain, and
timestamp details.

`signtool.exe` ships with the Windows SDK, for example:

`C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe`

## What a complete signed verification answers

1. **Signature present** — `Get-AuthenticodeSignature` is not `NotSigned`.
2. **Signer identity** — the certificate subject is the expected publisher.
3. **Timestamp** — a countersignature exists so the signature survives certificate expiry.
4. **Certificate chain** — Windows can build a path to a trusted root.
5. **SHA-256** — the file still matches the published hash.

All five are required before calling a public Windows build trustworthy. Today
only step 5 can pass, and only against an owner-provided hash.

## Files that will eventually need signatures

- `sammy.exe` (the application)
- `Sammy_0.1.0_x64-setup.exe` (NSIS installer)
- `Sammy_0.1.0_x64_en-US.msi` (MSI)
- the NSIS `uninstall.exe` that the installer writes onto the machine

Signing only the installer and leaving `sammy.exe` unsigned still produces
SmartScreen warnings at first launch.
