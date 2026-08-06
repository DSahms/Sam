# Memory Model — Auditable, Owner-Controlled

Sammy does **not** automatically convert conversation content into durable
memory. Anything the owner says in chat, and any reply Sammy gives, stays in
the conversation transcript and is forgotten when the conversation ends —
unless the owner explicitly promotes some piece of it into a knowledge
record. Memory is opt-in by default and auditable by construction.

## Why no auto-memory

Automatic memory systems quietly extract facts from chat and add them to a
long-term store. That pattern is incompatible with Sammy's promises:

- It hides what the system thinks it knows.
- It promotes guesses and confabulations to facts.
- It surfaces old, un-reviewed content in later retrieval.
- It makes "what does Sammy know about me?" unanswerable.

The auditable model replaces it: conversation is ephemeral-by-default,
memory is a candidate workflow, and every durable fact has a recorded
provenance and an owner decision attached.

## The memory-candidate workflow

A *memory candidate* is a proposed durable fact that has not yet been
promoted to Layer 1. Candidates arise only through an explicit action — the
owner marks a message as "remember this", or Sammy proposes a candidate and
asks the owner to decide. The owner retains total control.

```
conversation turn
        |
        v
[ candidate proposed ]   <-- explicit owner action OR owner-approved
        |                       Sammy suggestion
        v
[ candidate reviewed by owner ]
        |
   +----+----+----+----+----+----+----+
   |    |    |    |    |    |    |    |
 approve  edit  reject defer tmp corr super delete
   |    |+appr                                     |
   v                                              v
Layer 1 knowledge record                    discarded
```

## Candidate fields

Every memory candidate carries, at minimum, the following fields. They are
shown to the owner at review time and persisted as part of the audit trail
regardless of the decision.

- **proposed_text** — the wording to be remembered.
- **record_type** — fact, preference, instruction, correction, reference,
  etc.
- **source_conversation_id** — the conversation the candidate came from.
- **source_message_id** — the specific message (owner or Sammy) that
  produced it.
- **reason** — why this candidate was proposed (e.g., "owner said
  'remember'", "Sammy inferred a preference").
- **confidence** — Sammy's confidence the candidate is correct, 0..1.
- **sensitivity** — proposed sensitivity tag for the resulting record.
- **suggested_domain** — proposed domain for the resulting record.
- **suggested_retention** — proposed retention policy (e.g., indefinite,
  until a date, until superseded).
- **provider_used** — which provider produced the source message, if any.
- **source_crossed_to_cloud** — whether the source conversation turn was
  sent to a cloud provider (true/false). This determines whether the
  candidate is eligible for cloud-bound retrieval later.
- **timestamp** — when the candidate was proposed.

## Owner decisions

At review, the owner may take any of these actions. Each is recorded in the
audit log with the actor, the timestamp, and the candidate fields.

- **Approve.** Promote the candidate verbatim to a Layer 1 knowledge record.
- **Edit + approve.** Modify the proposed text or any of its metadata
  (sensitivity, domain, retention), then promote.
- **Reject.** Discard the candidate. **Rejected candidates are never used in
  retrieval, never indexed, and never appear in future context.** They are
  retained in the audit log only.
- **Defer.** Leave the candidate as a candidate for a later decision.
  Deferred candidates stay in the candidate queue and are **not** in
  retrieval.
- **Mark temporary.** Promote with a bounded retention (e.g., until a date
  or until the next supersession). Temporary records expire and become
  tombstones automatically.
- **Convert to correction.** Promote as a correction that supersedes one or
  more existing records; the superseded records gain a `superseded_by`
  pointer and stop appearing in retrieval.
- **Supersede.** Promote in place of an existing record; same effect as
  correction but driven by the owner identifying the older record.
- **Delete.** Remove a candidate outright with no record in the candidate
  queue. A delete of an *already-promoted* record is a tombstone, not a
  silent removal.

## Invariants

- Only approved (or edited-then-approved, or temporary, or correction, or
  supersede) candidates become Layer 1 records and enter retrieval.
- Rejected candidates never appear in retrieval, ranking, or context
  assembly, ever.
- Deferred candidates remain candidates; they do not appear in retrieval.
- Every promotion writes a provenance chain back to the source conversation
  and message, so the owner can always answer "where did Sammy learn this?".
- A record's `source_crossed_to_cloud` flag follows it into retrieval: a
  record promoted from a cloud-crossing turn is eligible to be sent to a
  cloud provider later only if the owner's routing rules permit.
- The candidate queue, the audit log of decisions, and the resulting
  knowledge records are all part of the corpus and are included in backups
  and exports (`CORPUS_CONTRACT.md`).

## Interaction with retrieval

Retrieval (`RETRIEVAL_ARCHITECTURE.md`) only ever sees approved records.
The candidate queue is invisible to it. This guarantees that nothing the
owner has not actively approved can color a future answer, and that the
retrieval pipeline never has to make a judgment call about an unreviewed
candidate.
