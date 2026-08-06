# Retrieval Architecture

Retrieval is the process that turns a user's question (and the surrounding
conversation) into the set of records and sources Sammy is allowed to place
into the model's context for the current turn. It runs entirely inside one
unlocked vault and never reaches across vaults.

## Components

- **SQLite FTS5 lexical search.** Built into the vault's SQLCipher
  database. Fast, deterministic, fully encrypted at rest with the vault.
- **Metadata filters.** Vault, permission, sensitivity, domain, recency,
  record-type, and source-ref filters expressed as SQL predicates applied
  *before* ranking.
- **Ranking.** A scoring step over the candidate set combining lexical
  relevance (BM25 from FTS5), optional vector similarity, recency, record
  quality, and domain match.
- **Dedup.** Collapse records that cite the same source range, and collapse
  near-identical records into a single representative with footnoted
  alternates.
- **Context budgeting.** Truncate the candidate set to a token budget
  computed from the active provider's context window minus the prompt,
  system instructions, and a reserved output margin.
- **Embedding provider interface.** A pluggable trait that turns text into
  vectors. Local embeddings (preferred) and cloud embeddings (audited) are
  both possible but must obey the same routing rules as chat providers.
- **Vector index interface.** A pluggable trait for storing and querying
  vectors. The index is per-vault, encrypted, rebuildable, and replaceable.
- **Local vector adapter.** The default vector adapter writes an encrypted
  sidecar index under per-vault storage and uses a local embedding model so
  no text leaves the device at index time.
- **Hybrid ranking.** When both FTS5 and a vector index are configured, the
  two candidate lists are merged and rescored so lexical precision and
  semantic recall reinforce each other.

## Canonical knowledge stays in SQLCipher

The vector index is a derived artifact, never the system of record. Every
vector in the index points back to a knowledge record ID and (optionally) a
source range in Layer 0. Deleting or rebuilding the index loses no
knowledge: it is regenerated from Layer 1. See `CORPUS_ARCHITECTURE.md`.

## Vector index is rebuildable

Rebuild is triggered:

- on first unlock after a provider or embedding-model change,
- after an import that touched records (`CORPUS_CONTRACT.md`),
- on owner request from settings,
- after detected index corruption.

A rebuild reads Layer 1 records and their referenced sources, recomputes
embeddings through the configured embedding provider, and writes a fresh
index. The old index is retained until the new one passes verification, then
atomically swapped and deleted.

## Required retrieval process (11 steps)

Every retrieval pass executes these steps in order. Skipping a step is a
bug.

1. **Validate that a vault is unlocked.** Refuse if no vault handle is
   active; refuse if the handle's session has expired.
2. **Analyze the query.** Determine intent, entities, and likely domains
   from the user's latest message plus rolling conversation context.
3. **Determine permitted domains and sensitivity ceiling.** Combine the
   vault's permission grants, the active routing mode, and any per-record
   sensitivity tags to produce the maximum sensitivity this turn may
   surface and the set of domains in scope.
4. **Lexical retrieval (FTS5).** Run a parameterized FTS5 query against the
   vault's index, scoped by the metadata filters from step 3. Return a
   ranked candidate list with BM25 scores.
5. **Vector retrieval (if configured).** Embed the query through the
   embedding provider, query the per-vault vector index with the same
   metadata filters, return a ranked candidate list with similarity scores.
   If no vector index is configured, skip this step.
6. **Merge and rank.** Combine the lexical and vector lists, apply hybrid
   ranking weights, and produce a single ranked list.
7. **Dedup.** Collapse records sharing a source range; collapse near-dupe
   records; keep the best-scoring representative and footnote alternates.
8. **Context budget.** Compute the remaining token budget for the active
   provider and truncate the ranked list to fit, preserving the top items
   and the diversity of domains.
9. **Assemble citations.** For each surviving item, build the citation
   metadata: stable record ID, source ID, source range, domain, sensitivity,
   confidence, recency. Citations are attached to the turn for later
   rendering and audit.
10. **Apply routing restrictions.** Re-check the surviving set against the
    routing mode: if the turn will cross to a cloud provider, strip any
    `local_only` records and recompute the budget; if that empties the set,
    either continue context-free or refuse per the routing mode.
11. **Send only permitted context.** Hand the trimmed, citation-tagged
    context to the provider layer. Nothing outside the surviving set is
    sent; nothing redacted is leaked via metadata.

## Filter precedence

Filters are applied in the database before ranking, in this precedence:

1. vault (implicit — only the active vault is queried)
2. permission (records the owner has not granted access to for this kind of
   action are excluded)
3. sensitivity (records above the turn's sensitivity ceiling are excluded)
4. domain (only domains in scope for the query are included)
5. recency (a configurable decay applied as a rank modifier, not a hard
   filter, except where the owner has set a hard cutoff)

## Failure modes

- **Unlocked vault lost mid-retrieval:** abort the turn, surface a re-unlock
  prompt, write no partial context to the provider.
- **Vector index missing or stale:** complete steps 1–4 and 6–11 against
  lexical-only results; schedule a rebuild; do not block the turn.
- **Embedding provider unreachable:** treat as vector-not-configured for
  this turn; do not fall back to a cloud embedding provider without an
  explicit owner-configured allowance.
- **Context budget zero after routing:** the provider is called with only
  the system prompt and the user's message; the turn is not silently
  enriched with forbidden records.
