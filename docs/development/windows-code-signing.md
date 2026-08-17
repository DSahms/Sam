# Windows code-signing readiness

[Release process](release-process.md) · [Verify packages](../getting-started/verify-windows-packages.md) · [External blockers](../../EXTERNAL_BLOCKERS.md)

This is a readiness plan, not a purchase. No certificate was bought, no paid
signing service was enrolled, and no production secret was generated.

## Current repository facts

`src-tauri/tauri.conf.json` has **no** `bundle.windows` signing block:

- no `certificateThumbprint`
- no `signCommand`
- no `digestAlgorithm` / `timestampUrl`

`Get-AuthenticodeSignature` on the current EXE, MSI, and NSIS artifacts returns
`NotSigned`.

The Windows SDK `signtool.exe` is present on the development PC. That is a
verification/signing *tool*, not a certificate.

`.gitignore` already drops `.env`, `*.pem`, `*.key`, `*.p12`, `*.pfx`, and now
also `.signing/`, `*.pvk`, and `*.spc`.

## A. Development / test signing

Safe to try locally, and **not** production trust:

- A self-signed certificate in a throwaway store, used only on the same PC.
- Signing a *copy* of an artifact in a temp folder, then running
  `Get-AuthenticodeSignature` / `signtool verify` to prove the commands work.
- Confirming that Tauri would pick up `certificateThumbprint` once a real
  cert exists — without committing the thumbprint until the release environment
  is ready.

Not safe to pretend:

- A self-signed file will silence SmartScreen for other people.
- A test certificate is a publisher identity.
- Checking in a `.pfx` “just for CI” is acceptable.

If you experiment, keep material under an untracked `.signing/` directory or a
machine certificate store. Never paste passwords into `tauri.conf.json`, docs,
or chat logs.

## B. Production signing

Sammy will eventually need one of these — chosen at purchase time, not now:

1. **Public CA Authenticode certificate** (standard or EV) in a hardware token
   or HSM, used with `signtool` / Tauri `certificateThumbprint`.
2. **Azure Trusted Signing** (or equivalent cloud signing) via Tauri
   `bundle.windows.signCommand`, where the private key never lands on the
   builder disk.

Vendor rules (identity checks, token drivers, cloud enrollment) change. Record
the vendor’s current requirements in the release environment when purchasing.
Do not copy vendor marketing into this file as if it were a repository fact.

What production must sign:

| Artifact | Why |
| --- | --- |
| `sammy.exe` | First launch and Defender inspect the application, not only setup |
| NSIS `Sammy_*_x64-setup.exe` | The file the user double-clicks |
| MSI `Sammy_*_x64_en-US.msi` | The file managed-install workflows double-click |
| NSIS `uninstall.exe` | Generated at install time; Tauri exposes `UNINSTALLERSIGNCOMMAND` |

Timestamp every signature (`signtool /tr <timestamp URL> /td sha256`) so
verification still works after the certificate expires.

## C. Secret storage

Keep signing credentials out of:

- git
- source (`tauri.conf.json` may later hold a **thumbprint**, which is an
  identifier, not the private key — still review it before committing)
- generated documentation and `STATE.md`
- build logs
- the packaged application

Allowed later in config: a certificate thumbprint and a public timestamp URL.
Never allowed: PFX paths with passwords, API keys, `.pfx` bytes, or
`signCommand` strings that embed secrets. Use environment variables or a
secret store, and keep a local `.signing/` directory gitignored.

## D. Verification

See [Verify Windows packages](../getting-started/verify-windows-packages.md).

Minimum production commands:

```powershell
Get-FileHash .\Sammy_0.1.0_x64-setup.exe -Algorithm SHA256
Get-FileHash .\Sammy_0.1.0_x64_en-US.msi -Algorithm SHA256
Get-AuthenticodeSignature .\Sammy_0.1.0_x64-setup.exe
Get-AuthenticodeSignature .\Sammy_0.1.0_x64_en-US.msi
Get-AuthenticodeSignature .\sammy.exe
signtool verify /pa /v .\Sammy_0.1.0_x64-setup.exe
```

Publish the SHA-256 list next to the files. Do not publish certificates’
private material.

## E. Release order

1. Clean source (no vaults, no `.env`, no signing files tracked).
2. Automated tests and quality gates.
3. Production build (`npm run tauri:build`) on the release machine.
4. Sign `sammy.exe` (and `sammy-cli.exe` if it is shipped beside it).
5. Generate installers **or** sign the installers Tauri just produced —
   whichever matches the chosen Tauri signing mode. If Tauri signs during
   bundle, do not also ad-hoc sign in a way that breaks the MSI tables.
6. Sign the NSIS uninstaller as required by the bundler.
7. Verify signatures and timestamps on EXE, MSI, and NSIS.
8. Publish a SHA-256 manifest for those three user-facing files.
9. Run [clean-Windows validation](../getting-started/windows-clean-install-validation.md)
   on a machine that is not the builder.
10. Only then publish the release.

Today the flow stops after step 3 plus unsigned hash inventory. Steps 4–7 are
blocked on a certificate. Step 9 is **UNVALIDATED**.

## Tauri knobs (not enabled)

When a real credential exists, Tauri 2 documents two Windows paths:

- `bundle.windows.certificateThumbprint` + `digestAlgorithm` (`sha256`) +
  `timestampUrl`
- `bundle.windows.signCommand` with `%1` replaced by the file to sign

Do not add those keys until the secret-handling path is decided. Adding an
empty thumbprint does not make builds signed.

## SmartScreen

Unsigned builds warn. Newly signed builds may still warn until the publisher
gains reputation. Signing is necessary for trustworthy distribution; it is not
a promise that SmartScreen will be silent on day one.
