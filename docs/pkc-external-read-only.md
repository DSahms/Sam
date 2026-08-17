# Sammy — external PKC (daily use)

Sammy's encrypted vault corpus is **not** the Personal Knowledge Corpus product.

```text
                    PKC
        durable personal knowledge
          facts / sources / provenance
                    |
         authorized stable boundary
          storykeeper | sammy | else deny
                    |
          +---------+---------+
          |                   |
          v                   v
 PKC Reference Client       Sammy
 proving/reference app    personal AI
```

## What enabling it does

Allows **authorized, read-only** retrieval for eligible **local** chat turns.
Sammy consults PKC as consumer `sammy` with purpose `personal_consigliere`.

## What enabling it does not do

- Does not upload PKC to a cloud model
- Does not automatically copy PKC into Sammy memory
- Does not give Sammy write or intake authority over PKC
- Does not query PKC on every message (greetings, arithmetic, and generic
  world facts are skipped)

## Setup (in Sammy)

1. Unlock a vault and open **Settings**.
2. Open **Personal knowledge (PKC)**.
3. Choose **Find local defaults**. It fills Python and Sammy's consumer/purpose
   when those are available. It does not guess a folder from another computer.
   Set the PKC location and bridge file on this machine if the fields are empty.
4. **Save**, then **Test connection**. Green **Authorized** means Sammy actually
   verified authorization. It does not dump corpus contents.
5. Enable the feature only after a successful test, then Save again if needed.
6. Chat normally on a local route (KoboldCpp or the local mock fallback).

Production default remains **off**. Configuration lives in the vault; restart
keeps the saved enable/disable state.

Advanced Python / bridge paths are under **Advanced connection details**. You
should not need to edit JSON files.

## Daily chat

On a local turn about you, your history, preferences, or documented facts,
Sammy may consult PKC, strip bookkeeping, and ground the local model. A subtle
**Used personal knowledge** control on the answer lets you inspect source
identity and evidence size — not raw infrastructure or private corpus dumps.

Cloud routing (including local→cloud fallback) does **not** retrieve PKC and
cannot carry previously retrieved PKC evidence into a cloud request. Retrieval
happens only after a local provider route is committed.

## Fact, testimony, and inference

- **Stored fact** — may be spoken as stored knowledge when the corpus supports it.
- **Testimony** — the owner's own words: “You previously described…”
- **Inference** — Sammy's reasoning: “That suggests…” — not source truth.

Sammy must not invent concrete scene detail (doorway, lighting, weather, and
similar) as if PKC supplied it.

## Retrieval judgment

Worth a lookup: questions about the owner, past events, decisions, preferences,
project history, relationships among stored information, recall/compare requests.

Not worth a lookup: greetings, thanks, pure arithmetic, generic world facts.

## Failure behavior

If Python, the bridge, or PKC is missing, unauthorized, slow, or malformed,
chat continues without personal knowledge. Settings shows an understandable
status. Diagnostics stay in logs/audit (identifiers, counts, hashes — not
corpus text).

## Diagnostic tool

`python tools/exercise_pkc_readonly_path.py --mode health|auth|retrieval|sanitize|local`

This is support infrastructure, not the user experience.

## Privacy model

PKC evidence is local-only. Cloud turns skip retrieval entirely. Retrieval
alone never writes Sammy durable memory and never creates a Memory Review
candidate.

To keep something, open **Used personal knowledge** on a grounded answer and
choose **Add to Memory Review**. That queues a short statement for review. It
becomes lasting Sammy memory only after you approve it. Editing or approving
that candidate does not rewrite PKC. Approved PKC-derived memories stay
`local_only` and follow Sammy’s existing cloud permissions.
