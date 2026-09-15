# Sam — Naming Convention

This is the authoritative naming map for the repository. It exists because the
project accumulated several generations of names (StoryKeeper, LedgerCore,
Sammy) across code, docs, and donor material, and an automated renaming pass
without rules is how history got tangled. **Any future session (human or AI)
must follow this file before renaming anything.**

Canonical names were confirmed by the owner on 2026-09-14.

---

## 1. Canonical names

| Canonical name | What it is | Where it lives |
|---|---|---|
| **Sam** | The personal assistant product. Default persona name. | `src/` + `src-tauri/` at repo root (moves to `apps/sam/` eventually) |
| **Calli Archiviste** | The personal knowledge intake engine — "the librarian". Accepts artifacts, interviews the owner, structures knowledge, submits to PKC. | `calli-archiviste/` (Flutter package, partially extracted) |
| **Calli Bench** | Minimal Windows desktop test bench running the real interview engine against KoboldCpp. Development tool only, never the product UI. | `apps/calli-bench/` (Stage 5-a) |
| **PKC** (Personal Knowledge Corpus) | The authoritative personal knowledge system — "the Dewey Decimal System". Software only; the owner's personal corpus data NEVER enters Git. | External repo (`F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus`); consolidates into `pkc/` eventually |
| **Media Archive** | Preservation layer for original artifacts (photos, letters, recordings). Evidence, never rewritten into knowledge. | `media-archive/` (not started) |
| **shared/** | Explicit cross-component contracts only. Not a dumping ground. | `shared/` (not started) |

---

## 2. Legacy names — banned from product code, UI, and runtime

The following names must NOT appear in new product code, UI strings, package
identities, or runtime constants:

- **Sammy** — legacy product name. New code/UI/docs use **Sam**.
- **StoryKeeper** — original donor application (now known to the owner as LedgerCore).
- **StoryKeeper-Local-Writer** — primary Calli donor.
- **LedgerCore / Ledger Series** — previous rename of the original StoryKeeper.
- **NURA**, **Persona-Lock** — separate products; must never be mixed into Sam.

### Where legacy names MAY still appear (exceptions, deliberately scoped)

1. **Donor provenance in `docs/migration/`** — migration docs must keep naming
   the donor paths (e.g. `D:\dev\StoryKeeper-Local-Writer\...`) so the pending
   extraction stays traceable. Donor provenance is documentation, not identity.
2. **Protected runtime strings** (do NOT rename unilaterally; each requires a
   coordinated change):
   - `app.sammy.desktop` — Tauri app-data identifier. Renaming it makes existing
     vaults invisible without a migration step. Deferred until after the 0.1.0
     release by owner decision.
   - `RULE-STORYKEEPER-PRIVATE-AUTOBIOGRAPHY` — grounding leak-filter marker
     present in **three in-repo consumers** (`src-tauri/src/grounding.rs`,
     `calli-archiviste/lib/services/grounding_policy.dart`,
     `tools/exercise_pkc_readonly_path.py`) and emitted by the external PKC
     gateway. Sam's filter deliberately carries BOTH generations
     (`RULE-SAMMY-*` and `RULE-STORYKEEPER-*`) so donor-era evidence still
     trips the leak check. Owner has instructed a full purge (2026-09-14);
     scheduled as a coordinated rename, NOT a docs-pass edit. Checklist when
     executed: rename in all three in-repo consumers in one commit AND update
     the gateway emitter in `F:\personal-knowledge-corpus-scaffold\...` in the
     same change, then re-run `python tools/exercise_pkc_readonly_path.py`
     (health/auth/sanitize modes) plus the forbidden-behavior suite as
     evidence.
   - `sammy.exe`, `sammy-cli`, `Sammy_0.1.0_*.msi/.exe` installer artifacts —
     rename in a dedicated release milestone, not in a docs pass.
   - `storykeeper_pkc_bridge.py` — filename of the external PKC gateway script
     that `PKCConfig.bridgeScript` resolves by default. Renaming it breaks the
     bridge path contract and the gateway emitter; rename only together with
     the gateway itself (same rule as the RULE-STORYKEEPER marker above).
   - Donor-provenance comments in `calli-archiviste/` — extracted donor code
     carries "Donor behavior note:" comments explaining why behavior is the
     way it is. These are documentation of contract behavior, not identity
     residue; they may be reworded only when the behavior note itself changes.
3. **Git history** — history is never rewritten to purge names.

---

## 3. Persona rule

**Sam** is the default assistant name, not a hard-coded identity. Onboarding
must let the owner name the assistant anything. The architecture must never
assume the persona string is "Sam" (identity is already provider-independent
via the `identity` module — keep it that way).

---

## 4. Donor protection

- `D:\dev\StoryKeeper` — reference/donor only. Never modified, never copied
  wholesale into Sam.
- `D:\dev\StoryKeeper-Local-Writer` — primary Calli donor. Never modified.
  Functionality migrates by surgical extraction only.
- `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus` — PKC
  software donor. Never modified merely because software is consolidated.
- Personal corpus data, artifacts, and interview evidence are **user data**:
  never in Git, regardless of where the software lives.

---

## 5. First-run onboarding shape (owner-confirmed 2026-09-14)

When Sam is ready for someone other than the owner:

1. First run ingests a **document dump** — bulk reference material about the
   new owner.
2. Once the owner feels enough reference material exists, **Calli** takes over
   intake: photos, audio clips, documents; Calli asks questions about them,
   the original artifact is preserved (Media Archive), and the structured
   data is placed where it belongs in the PKC.

The librarian interviews; the catalog stores; the archive preserves; Sam uses
all three. Naming in code must keep those four jobs distinguishable.
