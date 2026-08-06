# Sammy — North Star

This document captures the durable product intent. It changes slowly. Day-to-day
implementation decisions live in `DECISIONS.md`; status lives in `STATE.md`.

## The one-sentence product

Sammy is an encryption-first, local-first personal AI consigliere for Windows
desktops, whose durable product is the owner-controlled combination of identity,
knowledge, sources, memory, permissions, conversation history, audit history,
and personality configuration — with a replaceable language model.

## The single design principle

> Keep the soul. Change the brain.

Everything below is a consequence of this. The soul (identity, knowledge,
memory, relationships, history) is owner-controlled and provider-independent.
The brain (the language model and its provider) is replaceable, and replacing
it must never cost the owner their soul.

## Durable invariants

These must remain true at every phase, every release:

1. **Local-first.** The application is usable with no internet access. Network
   is used only for an explicitly selected cloud AI provider or local-model
   endpoint. No general startup call requires the internet.
2. **Owner-controlled keys.** Vault data-encryption keys are generated locally
   and wrapped by a passphrase the owner chooses (and a recovery key the owner
   saves). There is no vendor backdoor, no support override.
3. **No silent cloud.** Local-only data never goes to a cloud provider. No
   silent fallback across a privacy boundary. Every cloud transmission creates
   an audit event.
4. **No silent facts.** Model output is never automatically canonical
   knowledge. AI may propose; the owner approves. Conversation content never
   becomes durable memory without review.
5. **No silent action.** The application does not act externally without
   explicit permission. The first release executes no external actions at all.
6. **Replaceable brain.** Providers, extractors, and vector engines sit behind
   stable interfaces. Changing any of them must not reset identity, knowledge,
   memory, conversations, sources, or preferences.
7. **Independent vaults.** Vaults are fully isolated — separate keys, separate
   databases, separate sources, separate indexes, separate audit. No
   cross-vault retrieval. No shared private indexes.
8. **Honest security claims.** Sammy defends a documented, practical threat
   boundary and says so plainly. It does not claim perfect security, absolute
   anonymity, zero knowledge, or protection from an already-compromised OS.

## What Sammy is not

A generic chatbot wrapper. A coding agent. A therapy product. A medical
system. A surveillance system. A cloud-only service. A vector database
presented as a product. A system that silently converts AI guesses into facts.
A system that acts externally without explicit permission. A system that
uploads the owner's entire knowledge corpus to an AI provider.

## Out of scope for the first release

- macOS / Linux production packaging (platform code is isolated to allow later).
- Subscription billing, license activation, online accounts.
- Automatic multi-device sync, cloud replication, P2P sync, background remote
  transport (corpus moves via owner-controlled encrypted export/import).
- A network API server; external apps consume approved corpus export packages
  and a CLI validator, never the live vault.
- Telemetry, crash reporting, update checks, feature flags — none of these are
  present in the first release.
- External agentic actions (email, posting, purchases, file deletion, shell,
  browser automation). The permission architecture is built so these may be
  added later, but none execute in the first release.

## Success, honestly stated

Sammy is finished only when every acceptance criterion in
`docs/RELEASE_ACCEPTANCE.md` is demonstrated with evidence. A roadmap,
interface mockup, schema, or passing unit test alone does not satisfy the
definition of finished.
