# Vault Model — Multi-Vault Isolation

A **vault** is the unit of isolation in Sammy. Everything that makes Sammy
useful — sources, knowledge, conversations, indexes, identity, memory,
provider routes, permissions, audit log, and backups — lives inside exactly
one vault and cannot cross into another except through the deliberate,
audited, two-step transfer described below.

## What a vault owns

Each vault is a self-contained world. Creating a vault provisions, in
separate per-vault storage, all of the following:

| Asset                  | Scope                                   |
|------------------------|-----------------------------------------|
| SQLCipher database     | per vault                               |
| Data-encryption key    | per vault, random, independent          |
| Raw sources            | per vault                               |
| Conversations          | per vault                               |
| Knowledge records      | per vault                               |
| Runtime indexes (FTS5) | per vault                               |
| Vector indexes         | per vault                               |
| Identity               | per vault                               |
| Memory                 | per vault                               |
| Provider routes        | per vault                               |
| Permissions            | per vault                               |
| Audit log              | per vault                               |
| Backup history         | per vault                               |

There is no shared SQLCipher database, no shared key, no shared index, and no
shared provider routing state between vaults. Two vaults on the same machine
are strangers to each other by construction.

## Per-vault key

Every vault has its own random data-encryption key (see `KEY_MANAGEMENT.md`).
The key is generated at vault creation using the platform CSPRNG and is never
derived from any other vault's key, from the user's passphrase, or from any
machine identity. Because the key is independent:

- Compromising one vault's key compromises only that vault.
- Two vaults unlocked from the same passphrase do not share key material.
- A backup restored to a new machine does not need the original machine's
  keychain, OS account, or any online service.

## Hard isolation rules

The following are non-negotiable invariants enforced at every retrieval,
tool, and storage boundary:

1. **No cross-vault retrieval.** A retrieval pass scoped to vault A must
   never read from vault B's SQLCipher database, FTS5 index, or vector
   index. The retrieval layer is given exactly one vault handle per call.
2. **No shared embedding index.** Each vault has its own vector store. Even
   where the same embedding model is used, two vaults produce two physical
   index files in two different per-vault directories.
3. **No temporary grants.** There is no "let vault B see one record from
   vault A for this turn" mechanism. Such a feature would punch a hole in
   the isolation model and is explicitly out of scope.
4. **No shared audit log.** Audit events are written to the vault in which
   the action occurred. There is no global audit log.

## The only permitted transfer: audited export + separately confirmed import

Data may move between vaults by exactly one path:

```
[Source vault] --audited export--> package file --separately confirmed,
                                          audited import--> [Destination vault]
```

Properties of this transfer:

- **Two deliberate acts.** The export is one owner action in the source
  vault; the import is a *separate* owner action in the destination vault,
  requiring its own confirmation. There is no auto-import.
- **Both ends audited.** The source vault's audit log records the export
  (which records, when, to what path); the destination vault's audit log
  records the import (which records, from what source vault ID, when).
- **Package is opaque.** The exported package is encrypted with the
  destination's public fingerprint or with a passphrase chosen at export
  time; it is inert at rest.
- **Records are re-identified on import.** Imported records retain their
  original source vault ID in their provenance so the destination can see
  where they came from, but they receive destination-scoped stable IDs and
  become subject to the destination vault's permissions and routing rules.

Everything else — copy-paste of a record, drag between vault windows, a "send
to vault" button, a sync feature — is forbidden by this model until and
unless it is implemented as a variant of the export+import path above.

## Vault templates

At creation time the user picks a template. A template is **only** a set of
default labels and suggested settings; it carries **no data** and does not
link the new vault to any other vault. Templates do not share knowledge.

- **Personal** — default general-purpose vault. Suggested domains: personal,
  household, projects. Routing default: `ask_before_crossing`.
- **Witness** — optimized for recording and recalling observed events.
  Suggested domains: events, observations, evidence. Default routing:
  `local_only`. Drafts only by default for high-risk tools.
- **Consigliere** — advisory/decision-support configuration. Suggested
  domains: decisions, advice, context. Default routing: `prefer_local`.
- **Custom** — start from empty defaults; owner configures everything.

Picking a template writes only default settings and labels into the new
vault. It never copies a record, a source, a conversation, or an index from
any other vault, including a vault previously created from the same
template.

## Operational consequences

- Switching vaults in the UI is an unlock operation: the active vault is
  locked (its key is dropped from memory, its DB connection is closed, its
  indexes are unmapped) before the next vault is unlocked.
- Background work (index rebuild, backup) runs against a single explicitly
  unlocked vault; there is no "all vaults" job.
- A deleted vault purges its database, sources, indexes, and key material.
  Because nothing else references them, deletion is clean and complete.
