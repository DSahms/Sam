# Security boundaries

[Security model](../security/security-model.md) · [Threat model](../security/threat-model.md)

The principal boundaries are: locked versus unlocked vault; React versus Rust;
one vault versus another; canonical records versus derived indexes; local versus
cloud provider; and proposed versus approved memory.

Keys, credentials, database handles, provider authorization, retrieval filtering,
and restore replacement live behind Rust commands. CSP limits the webview to
self-hosted scripts/styles/images. The only audited unsafe Rust exception is the
Windows session-lock FFI; the crate otherwise denies unsafe code.

These controls do not defend an unlocked process from a fully compromised OS,
screen capture, keystroke logging, or a malicious model service selected by the
owner.
