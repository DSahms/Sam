# Windows security warnings

[Installation](installation.md) · [Verify packages](verify-windows-packages.md) · [Code signing](../development/windows-code-signing.md)

Sammy’s current Windows installers are **unsigned development packages**. Windows
is supposed to warn you. This page explains the warnings in ordinary language
and what to do instead of turning security off.

## Quick procedure

1. Confirm you received the installer from the project owner, not a random
   download site.
2. Check the file’s SHA-256 against the owner’s published value. See
   [Verify Windows packages](verify-windows-packages.md).
3. If Windows shows **Windows protected your PC**, **Publisher: Unknown**, or
   SmartScreen, treat that as expected for an unsigned build — not as proof the
   file is safe, and not as a reason to disable protection.
4. Continue only if the hash matches **and** you trust the person who gave you
   the file.
5. Never disable Smart App Control, Microsoft Defender, or SmartScreen
   machine-wide to “make Sammy install.”

If the hash does not match, do not install. Delete the file.

## What you may see

| Warning | What it is trying to say | What to do |
| --- | --- | --- |
| SmartScreen / “Windows protected your PC” | Windows has no reputation yet for this file or publisher | Verify the SHA-256; continue only if you trust the source |
| “Publisher: Unknown” / “unrecognized app” | The file is not Authenticode-signed, so Windows cannot name a company | Expected until Sammy is signed; not a green light |
| User Account Control (UAC) on MSI | The MSI installs for all users into Program Files and needs administrator approval | Expected for MSI. NSIS current-user install should not need this |
| Microsoft Defender scan | The file is being checked for known malware | Let the scan finish. Do not add blanket exclusions |
| WebView2 download prompt or progress | Windows needs the Edge WebView2 runtime to draw Sammy’s UI | Allow the official Microsoft download if the runtime is missing |

## Terms, in plain language

**SHA-256** is a fingerprint of the exact bytes in a file. If even one byte
changes, the fingerprint changes. Matching hashes means “this is the same file
the owner hashed,” not “this file is harmless.”

**Authenticode** is Microsoft’s name for a digital signature on a Windows
program. A valid signature answers “who published this?” and, with a timestamp,
“was it signed while the certificate was still valid?”

**A certificate** is a credential issued to a publisher. Sammy does not yet
have a production code-signing certificate. Unsigned ≠ automatically malicious.
Unsigned also ≠ trustworthy on the public internet.

**Timestamping** records *when* the file was signed. After a certificate
expires, a timestamped signature can still be trusted as “signed back then.”
Without a timestamp, an expired certificate makes the signature look stale.

**SmartScreen** is Windows reputation. Brand-new unsigned files look suspicious
because nobody has a history with them. Even signed files can warn until the
publisher builds reputation. An EV certificate or a cloud signing service can
shorten that, but reputation is still earned over time.

**MSI vs NSIS** are two installer technologies. Sammy ships both. NSIS currently
installs for your user account. MSI currently installs for the whole machine and
will show UAC. Neither format is “more secure” by itself; signing and the source
of the file matter more.

**UAC** is the administrator prompt. Seeing it for MSI is normal. Seeing it for
the NSIS current-user installer would be unexpected and should be recorded.

## What not to do

- Do not turn off SmartScreen, Smart App Control, or Defender to complete this
  test.
- Do not run `Unblock-File` as a substitute for checking the hash.
- Do not take a SmartScreen “More info → Run anyway” click as a security review.
- Do not treat a self-signed or test certificate as production trust.

Public distribution still requires a real code-signing plan. That plan is in
[Windows code signing](../development/windows-code-signing.md). It has not been
purchased or enrolled.
