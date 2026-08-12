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
