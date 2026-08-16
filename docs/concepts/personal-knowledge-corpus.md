# Personal Knowledge Corpus

[Documentation home](../README.md) · [Provenance](provenance.md) · [Corpus architecture](../architecture/corpus-architecture.md)

The Personal Knowledge Corpus **inside Sammy** is the durable, owner-controlled
knowledge layer in the encrypted vault. It is **not** the separate PKC product
that can be attached read-only in Settings.

[External PKC](../getting-started/external-pkc.md) is an independently
authoritative corpus. Enabling it does not merge the two systems. The vault
corpus still combines structured records, encrypted sources, provenance,
reviewed memory, conversation history, and audit history.

```mermaid
flowchart LR
    U["Owner"] --> S["Sources and approved facts"]
    S --> C["Canonical corpus in SQLCipher"]
    C --> I["FTS5 and rebuildable vector index"]
    I --> R["Filtered retrieval"]
    R --> X["Context assembly"]
    X --> L["Replaceable LLM provider"]
    L --> A["Response with provenance context"]
```

The model is not the authoritative memory store. Corpus records have stable IDs,
states, sensitivity, source references, and history. Models receive only the
context selected for a turn. Switching providers leaves canonical records intact.

Facts can be corrected through supersession, disputed through contradiction
state, or removed from current retrieval through tombstones. Vector embeddings
are deterministic derived data: useful for matching, but disposable and
rebuildable from current canonical records.
