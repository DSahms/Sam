# Memory, facts, and inference

[Memory Review](../user-guide/memory-review.md) · [Knowledge corpus](../user-guide/knowledge-corpus.md)

- **Fact/claim:** a structured corpus record with state and provenance.
- **Memory candidate:** a proposal awaiting owner review; not current truth.
- **Inference:** a model-generated conclusion that is not made durable merely by
  appearing in chat.
- **Conversation:** durable history, but not automatically retrievable knowledge.

Sammy’s review boundary prevents fluent model output from silently becoming
authoritative personal data. Approval is an explicit state transition; rejection
keeps the candidate out of retrieval.

External PKC retrieval is also not memory. Consulting PKC during chat writes
nothing durable and creates no review candidate. If you later choose
**Add to Memory Review**, the candidate keeps that provenance. Approving it
writes Sammy memory only; it does not rewrite PKC. A suggestion stays a
suggestion unless you rewrite the wording yourself.

