# Corpus Architecture

Sammy's *corpus* is everything the consigliere can read, reason over, and cite when
acting for a single user inside a single vault. The corpus is organized as four
strictly separated layers. Each layer has a different lifetime, a different trust
level, and a different storage technology. Code that crosses a layer boundary
must do so through a documented interface; it may never reach past a layer.

## The four layers

### Layer 0 — Raw Sources

The original bytes the user brought into the vault: PDFs, images, plain text,
audio captures, exported chat logs, screenshots, web clippings.

- **Storage:** encrypted files on disk, one file per source, addressed by a
  content-derived stable ID. Each source is encrypted with the vault's
  data-encryption key.
- **Trust:** lowest. Raw sources are unstructured, may contain duplicates, may
  be huge, and may include private material that should never be surfaced.
- **Purpose:** the material that *can* be parsed into Layer 1. Nothing in
  Layers 1–3 ever writes back into a raw source.

### Layer 1 — Curated Knowledge Records

The canonical, human- or owner-curated facts that Sammy is allowed to assert.
A record is the smallest unit of knowledge that can be approved, edited,
cited, superseded, or deleted.

- **Storage:** rows in the vault's SQLCipher database (`knowledge` table).
- **Trust:** high. Records have either been approved by the owner or come from
  a vetted import with a recorded provenance chain.
- **Purpose:** the authoritative memory of the vault. Every citation Sammy
  produces must resolve to a record in Layer 1 (or, when explicitly noted, to
  a raw source from Layer 0). Conversation content is **not** in Layer 1
  unless it has gone through the memory-candidate workflow (see
  `MEMORY_MODEL.md`).

### Layer 2 — Encrypted Runtime Search Indexes

The structures Sammy uses to find candidate records and sources quickly during
a retrieval pass: SQLite FTS5 lexical indexes and (optionally) per-vault
vector indexes.

- **Storage:** SQLite tables inside the same SQLCipher database as Layer 1
  (FTS5), plus optional sidecar vector index files that are also encrypted
  with the vault key.
- **Trust:** zero. Indexes are derived artifacts. They are never a source of
  truth and never appear in a citation.
- **Purpose:** speed. Indexes can be deleted, rebuilt, replaced, or swapped
  for a different embedding model without losing any knowledge.

### Layer 3 — Current Conversation Context

The working set assembled for a single turn: the in-progress messages, the
retrieved records/sources selected for this turn, the citations, and the
tool-call plan. This layer lives only in process memory while a conversation
is active and is persisted back to Layer 1 only through the explicit memory
or knowledge workflows.

## Why the canonical store is SQLCipher, not vectors

A vector index is an *approximation* optimized for similarity lookup. It is
lossy, model-specific, version-specific, and rebuildable. None of those
properties are acceptable for the thing the user trusts as their memory.

The canonical store is therefore the SQLCipher database: rows in the
`knowledge` table plus the encrypted raw source files referenced by row.
Vectors and FTS5 rows are projections of that canonical data.

Consequences of this decision:

- The vault can be unlocked, read, exported, and restored without any
  embedding model present.
- A different embedding model can be chosen later; the old index is dropped
  and rebuilt from Layer 1. No knowledge is lost.
- An FTS5 index corrupted by a crash is rebuilt from Layer 1 with a single
  command. No knowledge is lost.
- Backups and exports carry Layer 0 + Layer 1 + manifest, not the indexes.
  Restore re-derives Layer 2 on first run.

## Why indexes are rebuildable, replaceable, and per-vault

- **Rebuildable:** indexes never carry information that does not also exist in
  Layer 1. A rebuild reads Layer 1 records and their referenced sources and
  regenerates every FTS5 row and every vector.
- **Replaceable:** the embedding provider is a pluggable interface
  (`RETRIEVAL_ARCHITECTURE.md`). Changing providers changes the vector index
  format; the old index is discarded and a new one built.
- **Per-vault:** there is no global index. Two vaults on the same machine
  never share an FTS5 table or a vector store. This is a hard isolation
  boundary (`VAULT_MODEL.md`): cross-vault retrieval is forbidden and the
  storage layout makes it structurally impossible because each vault owns a
  distinct SQLCipher file with a distinct data-encryption key.

## Scale targets (per vault)

The corpus layer is designed to perform within these per-vault ceilings. Above
them the app must still function correctly, but latency is not guaranteed.

| Resource                          | Target ceiling |
|-----------------------------------|----------------|
| Documents (Layer 0 sources)       | 10,000         |
| Chunks (text indexed in FTS5)     | 250,000        |
| Knowledge records (Layer 1)       | 100,000        |
| Conversation messages (lifetime)  | 250,000        |
| Total source bytes (Layer 0)      | 100 GB         |

Engineering implications:

- **FTS5** must use an external-content table so the document text lives once
  in a content table and the FTS5 index references it. Rebuilds do not
  duplicate text.
- **Retrieval** must apply filters (vault, permission, sensitivity, domain,
  recency) inside the database before ranking, not after, to keep the
  candidate set small enough to rank at interactive latency.
- **Context budgeting** is mandatory: the retrieved set is truncated to a
  token budget before being placed into Layer 3, never dumped wholesale.
- **Vector index** must be paged off disk and not held resident; at 250k
  chunks with even a modest dimension it cannot fit comfortably in RAM
  alongside the LLM context.
- **Source dedup** by content hash at Layer 0 ingestion keeps the 100 GB
  target real; the same PDF imported twice stores bytes once and creates two
  source references.
