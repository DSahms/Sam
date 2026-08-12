# Frequently asked questions

[Documentation home](README.md) · [Troubleshooting](TROUBLESHOOTING.md)

**Is Sammy an AI model?** No. It is a desktop application and durable knowledge
layer that routes requests to replaceable models.

**Does it require Internet access?** No for vaults, corpus, documents, mock, or a
local KoboldCpp service. Venice requires Internet access.

**Can it run entirely locally?** Yes, with KoboldCpp and local-only routing.

**Does changing models erase memory?** No. Canonical data remains in the vault.

**Where is data stored?** Under `%LOCALAPPDATA%\app.sammy.desktop\vaults` by default.

**Is it encrypted?** Vault databases and source/configuration secrets are
encrypted. An unlocked compromised OS remains outside the protection boundary.

**What if KoboldCpp is offline?** The local call fails or development chat uses
the non-cloud deterministic mock; Sammy does not silently send to Venice.

**Can I move to another computer?** Yes, through encrypted backup and restore
using its passphrase or recovery code.

**Memory versus corpus?** Memory candidates are proposals. Only reviewed approval
promotes them into canonical corpus knowledge.

**What happens during document import?** Sammy extracts locally, encrypts the
original, records provenance/checksum, and indexes valid extracted text.

**Does Sammy train a model on my data?** No training pipeline is implemented.
Approved cloud requests may send selected context for inference.

**Can I delete information?** Sources can be soft-deleted and knowledge
tombstoned; both are excluded from active retrieval while audit/history semantics
are preserved.

**Which Windows versions are supported?** The first release target is Windows 11 x64.
