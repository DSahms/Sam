# Stage 5-b Design: Intake Registry + the Leeds Probe Chain

**Status:** Approved design (r1, re-landed 2026-09-16 after environment reset)
**Slice:** 5-b-1 (pure-Dart engine) — implemented in this commit
**Upstream spec:** `PKC_LAYOUT_SPEC.md` r2.1 (§5 record superset, §6 tiers, §7 intake contract)

Stage 5-b turns the interview from a chat into a **harvest**: every session walks
the Mother Leeds chain, every harvest mints candidates, and nothing becomes part
of the corpus until Dave personally stamps the gate. This document is the
contract for that machine.

---

## §1 The five levels

The chain is five questions deep, one level at a time, in order. The model may
attempt to walk all five; it may never force its way past a level.

| # | Level | Asks | Failure mode it catches |
|---|---------|------|--------------------------|
| L1 | surface | "Tell me about it." | The story itself, uninterrupted |
| L2 | sensory | "Pin one concrete detail." | Invented or borrowed scenery |
| L3 | source | "How do you know that?" | Secondhand memory wearing firsthand clothes |
| L4 | contrast | "What contradicts it?" | **The killer.** Registry may present approved records that contradict tonight's narrative |
| L5 | meaning | "Why does it matter?" | The point of keeping the memory at all |

L4 is where the registry earns its keep: it is the only level that can reach
into already-approved records and hold tonight's account against them.

Two rules govern the walk:

- The chain can be **attempted, never forced**. "I don't remember" is a legal
  answer at any level and is recorded as an explicit `skip`.
- Skipping is **visible forever**. Skips are recorded with level and reason and
  poison closure (§2, I-5).

> **Name origin (recorded 2026-09-16, per Dave):** the chain is named for the
> StoryKeeper-era probe in which Dave deliberately pushed the interview engine
> with the Mother Leeds story to see whether it would break character — and it
> did not. The name therefore honors the questioning ladder that held the
> persona lock under attack. It is also Dave's local lore: the Mother Leeds
> legend belongs to the New Jersey Pine Barrens (Leeds Point), which "brings
> the project home." The StoryKeeper identity itself remains purged per the
> Stage 0 decision; donor provenance stays in docs/migration.

## §2 State and the five invariants

`ProbeChainState` is the single mutable object a session walks with: topic,
mode, current level, pending question (with wording provenance), the turn list
(level, exact question, verbatim answer, timestamp), the skip list, and the
persona-discard audit trail. It serializes losslessly for the registry's
`sessions` table (§5).

The engine enforces five invariants. They are the difference between a chat log
and a corpus.

- **I-1 Escalation gate.** A level advances only on a substantive answer
  (non-blank; the engine judges emptiness, not quality — quality is the
  judgment layer's job) or an explicit skip. Blank answers re-ask in place,
  rotating quieter wording.
- **I-2 Closure rule.** Below contrast (L4), the code path cannot close a
  chain. The only exception is Dave closing manually with a `gate_log` stamp —
  which routes through the registry (5-b-2), never through engine state.
- **I-3 Verbatim fidelity.** Answers are stored exactly as received. No
  trimming, no normalization, no paraphrase — not by the engine, not by the
  model. Sanitization happens only in the export compiler (06_exports).
  Canonical text is never touched: STORE TRUTH / SANITIZE EXPORTS.
- **I-4 The engine cannot approve.** `mint()` produces `RecordCandidate`s and
  that is the ceiling. Nothing minted is approved, closed, or shipped; the gate
  is Dave's alone (§6).
- **I-5 Skip poisons closure.** Any skip at sensory/source/contrast (L2–L4)
  makes self-closure impossible even if the chain later reaches meaning (L5).
  The poison is visible in `state.skips` forever.

## §3 Dart contract (5-b-1 — implemented)

Three files in `calli-archiviste/`, pure Dart, no I/O, no model, no Flutter:

```dart
// lib/models/record_candidate.dart — mint table + gate policy
enum RecordStatus { candidate, approved, rejected, reopened, closed }
enum RecordType { event, judgment, claim, memoryCandidate }
class RecordCandidate { /* canonicalText, confidence, probeChain, sources */ }
class GatePolicy {
  static const String approvedActor = 'dave';
  static GateDecision evaluate({actor, action, currentStatus, recordType});
}

// lib/services/probe_chain_engine.dart — state machine
enum ElicitationMode { photo, document, audio, free_recall, chapter }
enum ProbeLevel { surface, sensory, source, contrast, meaning }
class ProbeTurn { level, question, answerVerbatim, at, questionSource }
abstract class ProbeChainEngine {
  ProbeChainState start({required ElicitationMode mode, required String topic});
  ProbeStepResult next(ProbeChainState s, String answer, {modelProposal, retryProposal});
  void skip(ProbeChainState s, String reason, {modelProposal, retryProposal});
  ClosureReport closureStatus(ProbeChainState s);
  List<RecordCandidate> mint(ProbeChainState s);   // candidates only (I-4)
}

// lib/services/question_banks.dart — offline Mother Leeds bank
class QuestionBank { static const motherLeeds = {...}; }
```

Design notes against the original r1 sketch: `next()` gained optional
`modelProposal`/`retryProposal` parameters so the persona-lock path
(discard → retry once → static fallback, §4) is engine-driven and testable
with mocks; the pending question (and its provenance) lives on the state, so
the Bench always knows what is on screen, including after `start()` and
`skip()`. Both are evolutions of the same contract, not replacements.

## §4 Division of labor

- **Engine owns state and decisions.** Deterministic, testable, runs with the
  model offline. Escalation, skips, closure eligibility, minting.
- **Model owns wording and persona.** At runtime the Bench asks the local
  model (Kobold, 127.0.0.1:5001, M40) to word the next level's question in
  Calli's voice. Proposals pass the **persona lock**: a small deterministic
  marker list catches break-character text ("as an AI", refusals, meta-talk).
  A discarded proposal is retried once, then the static bank stands. Every
  discard is audited in `state.personaDiscards`.
- **Static bank is the floor.** `question_banks.dart` carries Mother
  Leeds-pattern questions for all five levels plus re-ask variants. Model
  offline, persona lock tripping twice, or tests — the chain never stalls.
- **Dave owns approval.** The gate (§6) accepts exactly one actor.

## §5 Registry schema (5-b-2)

`registry.sqlite` is a **rebuildable index** over the JSON record files; the
JSON files are canonical (registry philosophy: files are truth, sqlite is
speed). One table is NOT rebuildable and is never exported: `gate_log`.

```sql
meta(key TEXT PRIMARY KEY, value TEXT);
sessions(session_id TEXT PRIMARY KEY, mode TEXT, topic TEXT,
         started_at TEXT, closed INTEGER, closure_report TEXT);
records(record_id TEXT PRIMARY KEY, record_type TEXT, status TEXT,
        canonical_text TEXT, sensitivity TEXT, pkc_tier TEXT,
        confidence REAL, source_sha256 TEXT, file_mtime TEXT,
        json_path TEXT);
records_fts(... )  -- FTS5, porter, unicode61, over canonical_text
mentions(record_id TEXT, mention_of TEXT, kind TEXT);
relationships(from_id TEXT, to_id TEXT, rel_type TEXT);
probe_closure(session_id TEXT PRIMARY KEY, reached_level TEXT,
              poisoned INTEGER, self_close_eligible INTEGER);
gate_log(ts TEXT, record_id TEXT, action TEXT, actor TEXT, note TEXT);
```

Write paths are exactly three: session append, gate action, rebuild. `rebuild`
is idempotent and reconstructs everything except `gate_log` from the JSON
files.

## §6 The gate

```
gate({recordId, action: approve|reject|reopen|close, actor})
```

- `actor != 'dave'` → denied, always, with the reason recorded.
- `close` requires status `approved` (I-2). The Dave-only manual override for
  below-L4 chains is a `gate_log` event with a note — it is a stamp on a
  decision, not a code path.
- Every accepted action appends to `gate_log` and updates the JSON record's
  `status` in the same transaction.
- `memory_candidate` records are engine-immutable: the engine never promotes
  them; only Dave's gate can.

## §7 Bench flow (5-b-3 / 5-b-4)

- Session screen: topic + mode chooser (photo / document / audio / free recall
  / chapter), then the chain. Five **level pips** across the top show where the
  chain stands — filled pips for substantive answers, hollow for current,
  struck-through for skips.
- Harvest review: after the chain ends, minted candidates are shown with
  confidence, probe chain walked, and provenance. Dave approves/rejects there;
  approval is a gate action, not a swipe.
- The L4 presentation question (registry contradictions live in-session vs at
  harvest) is deliberately left to §10 until Dave rules on it.

## §8 Test plan

1. `mint()` never returns approved/closed — candidates only, forever (I-4).
2. Gate rejects every actor but `dave`, including the engine itself.
3. Below L4, or with an L2–L4 skip, closure is refused (I-2 + I-5).
4. Golden fixture: full L1→L5 chain mints `event @1.0` + `judgment @1.0`,
   verbatim round-trip byte-exact (I-3).
5. Persona lock: break-character proposal discarded, retry once, static bank
   fallback; both discards audited.
6. Blank answer re-asks in place; skip escalates and records reason (I-1).
7. Registry rebuild from JSON is idempotent and never touches `gate_log`
   (5-b-2).

Items 1, 2, 3, 5, 6 (and the golden fixture from 4) are implemented in
`calli-archiviste/test/probe_chain_engine_test.dart`; item 7 lands with 5-b-2.

## §9 Slices

- **5-b-1 (this slice):** pure-Dart chain engine + question banks + record
  candidates + gate policy + tests. No I/O, no model, no F:.
- **5-b-2:** registry.sqlite writer/reader, gate transaction, rebuild, FTS.
- **5-b-3:** Bench session flow — chain UI, level pips, model wording with
  persona lock wired to Kobold.
- **5-b-4:** gate UI — harvest review, approve/reject/reopen/close, gate_log
  viewer.

## §10 Open questions (for Dave)

1. **E: backup facts** — path, date, and whether the existing backup contains
   the 622-piece golden set (unblocks export-compiler testing).
2. **L4 presentation** — do registry contradictions surface live in the
   session, or only at harvest review? (Recommendation: harvest only; keeps
   the session flow clean and the model out of the approval loop.)
3. **Helena as first live chain** — when 5-b-3 lands, does the Helena record
   (`39080ce5-…`) become the first chain walked end-to-end?
