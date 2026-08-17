# Memory Review

[Knowledge corpus](knowledge-corpus.md) · [Memory and inference](../concepts/memory-vs-inference.md) · [External PKC](../getting-started/external-pkc.md)

Memory Review is the deliberate bridge between conversation-derived candidates
and durable knowledge. Pending candidates can be approved, edited then approved,
rejected, deferred, marked temporary, or deleted.

Approval creates a knowledge record with provenance. Rejected candidates never
enter retrieval. Deferred and temporary states remain distinct from current
truth. There is no automatic “the model said it, therefore Sammy knows it” path.

Use **What I Know** for direct owner-authored facts and Memory Review when a
conversation suggests something worth preserving.

## Personal knowledge (PKC)

Consulting PKC during chat is temporary and read-only. It does **not**
automatically teach Sammy lasting memory.

If a local answer used personal knowledge, **Add to Memory Review** queues a
short statement you choose. The candidate shows:

- the proposed text
- whether it came from personal knowledge
- whether it is stored knowledge, something you described, or a suggestion
- source identity (not the full source document)

A suggestion does not become a stored fact merely because you queued or
approved it. Editing the candidate changes the new Sammy memory, not the PKC
source. Duplicate pending statements reuse the existing candidate. If Sammy
already has a different lasting memory from the same source, review shows the
conflict and does not pick a side for you.

Approved PKC-derived memories are local-only. Turning PKC off, or PKC going
offline, does not remove those approved memories or pending review items.
