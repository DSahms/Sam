# Corpus and vector architecture

[Corpus concept](../concepts/personal-knowledge-corpus.md) · [Existing corpus design](../CORPUS_ARCHITECTURE.md)

Canonical knowledge lives in versioned SQLCipher tables. FTS5 supports lexical
matching. `SqliteVectorIndex` stores deterministic fixed-size vectors in the same
encrypted database and computes cosine similarity during search.

Hybrid retrieval merges lexical and vector candidates, filters record state and
sensitivity, deduplicates, ranks, applies a context budget, and returns stable
record IDs for prompt context. Adds, corrections, tombstones, corpus imports, and
memory approvals update the index. A rebuild clears derived rows and regenerates
them from current canonical records.

Corpus JSON export/import validates versions, checksums, and unique IDs and is
idempotent. Full export is implemented; incremental export remains a future
enhancement described by the corpus contract, not a shipped UI workflow.
