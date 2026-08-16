# Knowledge corpus

[Corpus concept](../concepts/personal-knowledge-corpus.md) · [Documents](document-ingestion.md) · [Memory](memory-review.md)

**What I Know** manages structured, durable knowledge rather than raw chat text.
Owners can add facts, filter and search records, approve or reject candidates,
create corrections, and tombstone information.

Corrections supersede rather than silently overwrite history. Contradictory
claims can coexist with explicit dispute state. Tombstoned, rejected, and
superseded records are excluded from current retrieval while history and
provenance remain available for audit.

The SQLCipher records are canonical. FTS5 and vectors are derived indexes and can
be rebuilt; deleting an index does not delete the knowledge it represents.

This vault corpus is **not** the external Personal Knowledge Corpus product.
Optional read-only PKC access is configured in Settings and does not copy PKC
records into **What I Know**. See [external PKC](../getting-started/external-pkc.md).
