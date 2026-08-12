# Logs and diagnostics

[Troubleshooting](../TROUBLESHOOTING.md) · [Privacy controls](../user-guide/privacy-controls.md)

Use **Privacy & Audit** first: it exposes structured event categories and recent
events within the unlocked vault. Provider connection tests return sanitized
user-facing status. Build diagnostics appear in the invoking development shell.

Do not paste vault databases, manifests, backup packages, API keys, recovery
codes, or full private prompts into support reports. Useful safe details include
Sammy version, installer type, Windows version, routing mode, provider class,
endpoint host/port for localhost, exact sanitized error, and the failing command.

Linker `LNK4099` warnings about `ossl_static.pdb` are expected missing debug-symbol
metadata in vendored OpenSSL builds when the command still exits successfully.
