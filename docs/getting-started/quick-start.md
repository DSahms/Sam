# Quick start

[Documentation home](../README.md) · [Installation](installation.md) · [KoboldCpp](koboldcpp.md)

This is the shortest path from an installed application to a real local
conversation. You need Sammy, a Windows 11 computer, and KoboldCpp running with a
model loaded. “Localhost” means a service running on this same computer; `5001`
is the numbered network port where Sammy expects to find it.

1. Open **Vaults**, create a vault, and save the one-time recovery code separately.
2. Start a KoboldCpp OpenAI-compatible API, normally at `http://localhost:5001`.
3. In **Settings**, enter the endpoint, test it, choose a discovered model, enable
   KoboldCpp, and save.
4. Open **Chat**, create a conversation, check the provider/model badge, choose a
   routing mode, and send a message.
5. Use **What I Know** for approved facts, **Sources** for documents, and **Memory
   Review** for deliberate promotion of conversation-derived candidates.

You have succeeded when Chat shows the selected KoboldCpp provider/model and a
reply appears without a cloud-consent prompt. The corresponding backend result is
marked `crossed_to_cloud: false`.

If you do not yet have KoboldCpp, you can still explore vaults, sources, knowledge,
memory, backup, and the deterministic Mock provider. Mock responses verify Sammy's
workflow but are not answers from a language model.

Next: [Chat](../user-guide/chat.md) · [Knowledge corpus](../user-guide/knowledge-corpus.md) · [Backup](../user-guide/backup-recovery.md)
