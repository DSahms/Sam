# Sam — Project Master Architecture & Vision

**GitHub:** https://github.com/DSahms/Sam  
**Local workspace:** `C:\Users\david\ZCodeProject\Sammy`  
**Baseline commit:** `cf844651d73ae1fcbf9aab433423731d26fe4636`  
**Commit:** `chore: establish Sam architecture checkpoint`

> This document is the master description of the Sam ecosystem: its purpose, architecture, component responsibilities, artifact/knowledge flow, migration boundaries, privacy rules, and long-term direction.

---

## 1. The Project in One Sentence

**Sam is a personal AI assistant built around Calli Archiviste, a personal knowledge intake engine; PKC, the authoritative personal knowledge system; and a separate Media Archive that preserves the original evidence from which knowledge is derived.**

The core principle is:

> **The original artifact, the knowledge derived from it, the conversation used to collect it, and the AI assistant that uses it are different things. They can be connected, but they must not be confused.**

---

## 2. The Overall Architecture

```text
                         SAM
                 Personal Assistant
                         |
              +----------+----------+
              |                     |
              v                     v
      CALLI ARCHIVISTE              PKC
   Personal Knowledge Intake   Authoritative Knowledge
           Engine                    System
              |                     ^
              |                     |
              v                     |
        MEDIA ARCHIVE -------------+
        Original Artifacts
```

### Component responsibilities

| Component | Responsibility |
|---|---|
| **Sam** | Personal assistant and conversational interface |
| **Calli Archiviste** | Personal knowledge and artifact intake/interview engine |
| **PKC** | Authoritative personal knowledge, provenance, evidence, retrieval, authorization |
| **Media Archive** | Preservation of original artifacts |
| **Shared** | Explicit cross-component contracts/utilities only |
| **AI model** | Reasoning/generation; not authoritative personal storage |

This is a **single GitHub repository/monorepo**, not a collection of child repositories.

---

# 3. Sam

## What Sam Is

Sam is the primary personal AI assistant.

Sam is responsible for:

- conversation
- reasoning and assistance
- retrieving authorized personal knowledge
- deciding when specialized intake is useful
- invoking Calli when the user wants to add or contextualize knowledge
- presenting knowledge back to the user
- eventually coordinating other personal-assistant capabilities

Sam should not own the entire personal knowledge database internally. It should consume authoritative knowledge through defined interfaces.

## Identity

The current product/default assistant name is **Sam**.

The architecture should eventually allow the user to rename the assistant/persona without restructuring the system. Therefore `Sam` is the current product identity, not a permanent hard-coded persona identity.

`Sammy` is legacy naming. New application code, UI, documentation, and identifiers should use **Sam**.

The local outer directory may temporarily remain `Sammy` and should not be renamed until the application is stable.

---

# 4. Calli Archiviste

## Meaning

**Calli Archiviste** is the human-facing name of the Personal Knowledge Intake Engine.

The name is inspired by Callimachus of Alexandria, associated with cataloging and organizing knowledge, and the French word *Archiviste*, meaning archivist.

The intended metaphor is:

> **Calli is the user's personal librarian.**

Calli talks to the user, asks questions, gathers context, organizes information, and prepares durable knowledge for PKC.

Calli is broader than a writing application.

---

# 5. What Calli Does

Calli handles **intake**.

Typical flow:

```text
User presents artifact or information
              |
              v
       Calli identifies it
              |
              v
       Calli interviews user
              |
              v
  Who / What / When / Where / Why
              |
              v
     Meaning and personal context
              |
              v
     Structured knowledge package
              |
              v
          PKC intake
              |
              v
    Authoritative stored knowledge
```

Calli should support artifacts such as:

- photographs
- scanned documents
- letters
- audio
- video
- project files
- other personal artifacts

Questions may include:

- Who is in this?
- When was this taken?
- Where was this?
- What was happening?
- Who else was involved?
- What happened before or after?
- Why does this matter?
- What does this artifact mean to you?
- What should be remembered about it?
- What source supports this information?

The objective is **structured, attributable knowledge**, not merely a nice description.

---

# 6. What Calli Is Not

Calli is not primarily:

- a memoir-writing application
- a book generator
- a long-form narrative compiler
- a print-layout application
- a general publishing system
- a replacement for Sam
- the authoritative PKC database
- the original artifact archive

Writer-oriented functionality that existed in StoryKeeper-Local-Writer does not automatically belong in Calli.

Examples to exclude unless deliberately added later:

- book formatting
- print services
- long-form narrative compilation
- memoir output
- book providers
- narrative-provider-specific functionality

---

# 7. PKC — Personal Knowledge Corpus

PKC is the authoritative personal knowledge layer.

PKC is responsible for:

- structured knowledge
- provenance
- evidence
- retrieval
- authorization
- security boundaries
- knowledge relationships
- source references
- durable personal knowledge

PKC answers:

> **What does the system officially know, and what evidence supports it?**

Calli answers:

> **How do we collect and organize what the user wants the system to know?**

Sam answers:

> **How can I use that knowledge to help the user?**

---

# 8. PKC Software vs Personal Data

This distinction is critical.

## PKC software

The application/code implementing things such as:

- intake
- retrieval
- evidence handling
- authorization
- knowledge storage
- search
- APIs/interfaces

The software may eventually be consolidated under:

```text
Sam/pkc/
```

## Personal PKC data

The actual private corpus, including:

- photographs
- documents
- audio
- manuscripts
- personal records
- personal knowledge database
- raw archives
- secrets
- private files

This data is **not source code** and must not be copied into Git merely because PKC software is consolidated.

GitHub should contain software and appropriate documentation, not the user's private personal corpus.

---

# 9. Media Archive

The Media Archive preserves the **original artifact**.

It is deliberately separate from knowledge derived from that artifact.

```text
Original photograph
        |
        v
Media Archive
        |
        +--> stable artifact ID
        +--> original file
        +--> hash
        +--> provenance
        +--> metadata
        |
        v
Calli interviews user
        |
        v
Structured knowledge
        |
        v
PKC
```

The original artifact remains evidence. PKC knowledge references it.

---

# 10. Artifact vs Knowledge Boundary

## Artifact

The original thing:

- image
- scanned letter
- audio recording
- original document
- video

## Knowledge

What the system learns about it:

- who is pictured
- when it was taken
- where it was taken
- what happened
- relationships
- historical context
- personal meaning
- user statements

The artifact should not be rewritten into knowledge. Knowledge should reference the artifact.

---

# 11. Provenance

Personal knowledge needs provenance.

Whenever practical, PKC should distinguish:

- where a fact came from
- whether the user stated it
- whether it came from an artifact
- whether it was inferred
- whether it was verified
- what evidence supports it
- when it entered the corpus
- what artifact or conversation produced it

The system should not silently turn an inference into a verified fact.

Conceptually:

```text
User statement
      |
      v
Captured knowledge
      |
      +--> source
      +--> evidence
      +--> provenance
      +--> verification status
```

---

# 12. Importing Files and Artifacts

The intended import workflow is:

### Step 1 — Select/import

The user gives Sam or Calli an artifact.

### Step 2 — Preserve the original

The original artifact enters the Media Archive or approved archival workflow without altering the original contents.

### Step 3 — Identify

Calli creates an artifact reference and gathers metadata such as:

- artifact ID
- filename
- media type
- timestamp if known
- file hash
- source
- import date
- provenance
- related artifacts

### Step 4 — Interview

Calli asks adaptive questions about the artifact and the user's personal context.

### Step 5 — Structure

Calli turns the interview into structured knowledge.

### Step 6 — Submit to PKC

Calli sends a defined knowledge/intake DTO to PKC rather than manipulating PKC internals.

### Step 7 — Link

PKC knowledge retains a reference to the original artifact.

```text
Artifact <----> Media Archive
    ^
    |
    | artifact reference
    |
   PKC
    ^
    |
 Calli
    ^
    |
   Sam
```

---

# 13. Example: Photograph Intake

Suppose the user imports 200 old photographs.

The system should not simply dump them into a vector database.

### Archive

Preserve each original photograph.

### Identify

Assign stable artifact references.

### Interview

Calli asks who, what, when, where, what happened, and why it matters.

### Extract

Capture people, location, date, event, relationships, circumstances, and significance.

### Store

PKC stores structured knowledge and provenance.

### Link

PKC references the original photograph.

Later Sam can answer questions such as:

> "What do I know about the family reunion in the late 1970s?"

and retrieve authoritative knowledge while still being able to reference the original artifacts.

---

# 14. Documents and Audio

A scanned letter follows the same principle:

```text
Original letter
      |
      v
Media Archive
      |
      v
Calli
      |
      +--> who wrote it?
      +--> who received it?
      +--> when?
      +--> where?
      +--> why is it important?
      +--> what does it establish?
      |
      v
PKC
```

An audio recording similarly remains preserved while transcripts, identified speakers, context, important statements, and personal meaning can become derived knowledge.

A transcript does not automatically replace the original recording.

---

# 15. Calli → PKC Contract

The Calli-to-PKC boundary should be a clean contract.

Calli may provide:

- artifact reference
- context
- extracted facts
- user statements
- provenance
- evidence references
- interview/session information
- confidence or verification metadata where appropriate

PKC owns:

- authoritative storage
- validation
- authorization
- evidence relationships
- retrieval
- durable knowledge identity
- security rules

Calli should not become tightly coupled to PKC implementation details.

A DTO/interface boundary is preferred.

---

# 16. Sam → Calli → PKC

```text
                         USER
                           |
                           v
                         SAM
                    Personal Assistant
                           |
               "I want to remember this"
                           |
                           v
                    CALLI ARCHIVISTE
                 Intake / Interview Engine
                           |
                +----------+----------+
                |                     |
                v                     v
         MEDIA ARCHIVE               PKC
        Original Artifact       Structured Knowledge
                                      |
                                      v
                                  Evidence /
                                  Provenance
```

Sam should invoke Calli through a defined interface rather than depending directly on Calli internals.

---

# 17. Interview Engine

The primary donor for the enhanced interview/intake implementation is:

```text
D:\dev\StoryKeeper-Local-Writer
```

The original StoryKeeper application is **not** the new Calli application.

A key legacy dependency was:

```text
package:storykeeper/services/interview_engine.dart
```

A migration objective is to remove this dependency by extracting the actual required interview functionality.

There must be **exactly one authoritative Calli interview engine**.

---

# 18. StoryKeeper Boundaries

Original StoryKeeper:

```text
D:\dev\StoryKeeper
```

is protected reference/donor material.

It must not be modified during migration and must not be copied wholesale into Sam.

StoryKeeper's identity should not survive as Calli's identity.

The goal is to extract useful functionality, not resurrect the old application under another name.

Primary Calli donor:

```text
D:\dev\StoryKeeper-Local-Writer
```

Only functionality required for personal knowledge intake should migrate.

The donor itself should remain protected.

---

# 19. Storage Boundary

Application state and authoritative knowledge must remain separate.

### Application/session state

Examples:

- current interview session
- UI state
- temporary workflow state
- local preferences
- navigation state
- draft interview state

### Knowledge

Examples:

- facts
- relationships
- memories
- provenance
- artifact context
- user statements intended for long-term retention

### Artifacts

Examples:

- photos
- documents
- recordings

### Generated output

Examples:

- narratives
- drafts
- formatted books

These categories should not be silently merged.

---

# 20. Local Storage

Legacy Local Writer storage may contain a mixture of session state, messages, application state, and knowledge-like material.

It must be classified rather than blindly copied.

Conceptually:

```text
Application / UI state
        |
        v
local session storage

Long-term knowledge
        |
        v
PKC

Original artifacts
        |
        v
Media Archive
```

Not everything in local storage belongs in PKC.

---

# 21. AI Models and Providers

The system may use local or remote language models. The model/provider is an implementation detail.

The architecture should not make the knowledge system dependent on one model provider.

The model generates or interprets information; **PKC remains the authoritative knowledge layer**.

---

# 22. Shared Directory

`shared/` is not a dumping ground.

Code belongs there only when there is a concrete cross-component contract or utility.

Potential future contents include:

- DTO definitions
- stable IDs
- shared protocol definitions
- serialization formats
- cross-component interfaces

Do not move code there merely because two components might someday use it.

---

# 23. Repository Architecture

The intended monorepo is:

```text
Sam/
├── apps/
│   └── sam/
├── calli-archiviste/
├── pkc/
├── media-archive/
├── shared/
└── docs/
    ├── architecture/
    ├── migration/
    └── decisions/
```

These are directories inside **one GitHub repository**.

There is no requirement to create separate GitHub repositories for Calli, PKC, Media Archive, or Shared.

---

# 24. Current Repository Reality

The repository began as the existing Sam/Tauri application. The application still has its existing root-level implementation, including:

```text
src/
src-tauri/
```

The new architecture directories were created before all implementation code was migrated.

That transition is intentional.

The project should progressively extract and move functionality into the new boundaries rather than performing a blind whole-repository rewrite.

---

# 25. GitHub Baseline

The authoritative GitHub repository is:

```text
https://github.com/DSahms/Sam
```

Local workspace:

```text
C:\Users\david\ZCodeProject\Sammy
```

The first architecture checkpoint has been pushed successfully.

```text
Commit: cf844651d73ae1fcbf9aab433423731d26fe4636
Message: chore: establish Sam architecture checkpoint
Branch: main
Remote: origin -> https://github.com/DSahms/Sam.git
```

This checkpoint is the baseline for continuing migration.

---

# 26. Git Strategy

Sam is the authoritative application monorepo.

Do not create separate repositories for Calli, PKC software, Media Archive, or Shared unless a deliberate future architectural decision requires it.

Use meaningful commits at migration milestones so the system can always be rolled back to a known-good state.

---

# 27. Protected Donors

### Original StoryKeeper

```text
D:\dev\StoryKeeper
```

Reference/donor only.

### StoryKeeper-Local-Writer

```text
D:\dev\StoryKeeper-Local-Writer
```

Primary Calli donor.

### External PKC

```text
F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus
```

PKC software source/donor.

The external PKC repository should not be modified merely because PKC software is eventually consolidated.

Personal corpus data must remain outside Git.

---

# 28. Migration Anti-Patterns

Do not:

1. blindly copy the entire Local Writer application
2. copy the original StoryKeeper application
3. copy personal PKC/archive data into Git
4. blindly copy the old PKC bridge
5. create another duplicate interview engine
6. put everything into `shared/`
7. perform global search/replace
8. preserve StoryKeeper identity in Calli
9. mix NURA into Sam
10. mix Persona-Lock into Sam
11. treat generated build artifacts as source
12. move private databases into Git
13. make Sam depend directly on Calli internals
14. make Calli depend directly on PKC internals
15. treat AI-generated narrative as authoritative knowledge without provenance

---

# 29. Knowledge Authorization

Not every conversational statement should automatically become permanent knowledge.

A useful conceptual pipeline is:

```text
Conversation
     |
     v
Candidate information
     |
     v
Calli intake / structuring
     |
     v
Authorization / provenance
     |
     v
PKC authoritative knowledge
```

The eventual system should make it possible to determine what was captured, why it was captured, where it came from, whether it was authorized, whether it was verified, and what evidence supports it.

---

# 30. Future User Experience

The architecture should eventually feel simple to the user.

For example:

```text
User: "Who is in this picture?"

Sam: "I can help identify it. Let me open it in Calli."

Calli: "Who is the person on the left?"

User: "That's my grandfather."

PKC: stores structured knowledge + provenance + artifact reference

Sam: "That's your grandfather, according to the information you recorded
about this photograph."
```

The user should not need to understand the internal architecture to use it.

---

# 31. Long-Term Vision

The long-term system is an always-available personal assistant grounded in the user's own authoritative knowledge.

It should eventually understand and retrieve:

- personal history
- people and relationships
- projects
- documents
- photographs
- memories
- preferences
- important events
- artifacts
- personal context
- how the user reacts to things

The goal is not merely:

> "Make an AI that knows me."

The goal is:

> **Build durable personal knowledge infrastructure that an AI assistant can safely use.**

---

# 32. Security and Privacy Philosophy

Personal knowledge is sensitive.

The architecture therefore favors:

- local-first storage where practical
- explicit authorization
- provenance
- separation of personal data from source code
- encrypted/private storage where appropriate
- stable artifact references
- controlled retrieval
- clear consumer permissions

The GitHub repository contains software and documentation.

The private personal corpus remains private.

---

# 33. Current Status

At the baseline checkpoint:

- Sam parent repository exists.
- GitHub repository exists.
- `main` has been pushed.
- Architecture directories have been created.
- Architecture documentation exists.
- Migration documentation exists.
- A Calli foundation/checkpoint exists, but full extraction is not complete.
- PKC software consolidation is not complete.
- Media Archive implementation is not complete.
- Full Sam/Calli/PKC integration is not complete.

This is a **baseline**, not the finished product.

---

# 34. Recommended Development Sequence

```text
GitHub checkpoint
       |
       v
Calli extraction
       |
       v
Remove StoryKeeper dependency
       |
       v
Calli -> PKC contract
       |
       v
Media Archive boundary
       |
       v
PKC software consolidation
       |
       v
Sam integration
       |
       v
End-to-end testing
       |
       v
Release-ready architecture
```

Work incrementally and commit at meaningful milestones.

---

# 35. Definition of Success

## Sam

A functioning personal assistant that can retrieve and use authorized personal knowledge.

## Calli

An intake engine that can accept artifacts/information, interview the user, structure the resulting knowledge, preserve provenance, and submit it to PKC.

## PKC

The authoritative source for durable personal knowledge.

## Media Archive

A reliable preservation layer for original artifacts.

## Provenance

The system can explain where important knowledge came from.

## Security

Private personal data is not accidentally placed in source control.

## Architecture

The components have clear responsibilities and clean interfaces.

## Repository

One GitHub repository contains the software architecture without requiring unrelated child repositories.

---

# 36. North Star

> **Give Sam your life, memories, documents, photographs, projects, and context — and Sam should be able to remember, understand, retrieve, and use that information responsibly because the knowledge underneath it has been deliberately captured, organized, sourced, and preserved.**

---

# 37. Master Component Summary

```text
SAM
  Personal assistant
  Conversational interface
  Uses authorized knowledge
  Invokes Calli when intake is needed

CALLI ARCHIVISTE
  Personal librarian
  Artifact intake
  Interview engine
  Context gathering
  Structured knowledge preparation
  Provenance capture
  PKC submission

PKC
  Authoritative knowledge
  Evidence
  Provenance
  Retrieval
  Authorization
  Security

MEDIA ARCHIVE
  Original artifacts
  Artifact IDs
  Hashes
  Original-file preservation
  Artifact metadata

SHARED
  Explicit contracts only

AI MODELS
  Reasoning and generation
  Never the authoritative personal knowledge store
```

---

# 38. Final Architectural Rule

If a future change creates uncertainty, return to these four questions:

1. **Is this the assistant?** → Sam
2. **Is this collecting/organizing personal knowledge?** → Calli
3. **Is this authoritative durable knowledge?** → PKC
4. **Is this the original evidence/artifact?** → Media Archive

If it does not clearly belong to one of those boundaries, do not move it automatically. Define the interface first.

---

# Addendum — 2026-09-14 (owner-confirmed in working session)

> Appended after the baseline checkpoint. Nothing above this line was edited.
> This addendum records decisions and vision elements stated by the owner in
> the working session of 2026-09-14, so future sessions inherit them.

## A1. Purpose

The deepest motivation for Sam is **memory preservation**. The owner has lost
29 friends since 2019; for the most part they live on only in memory. The
project exists so that neither the owner's memory nor the memory of those
friends "has to die again." The governing image: a handprint pressed on a cave
wall 2,000 years ago still tells us someone was there. The Media Archive is
the handprint; the PKC is everything we can still say about the person, kept
honest by provenance. Practical consequences for design:

- Artifacts (photos, recordings, letters) of friends and family are first-class
  Media Archive citizens, and structured memories derived from them are
  first-class PKC records with provenance.
- The assistant must never silently degrade remembered testimony into
  invented fact (already a durable invariant: "No silent facts").

## A2. Naming decisions confirmed

- Purge **StoryKeeper / LedgerCore / Ledger Series** identity from product
  code, UI, and runtime strings in the owner's version. Donor *provenance*
  inside `docs/migration/` remains. See `NAMING.md` for the full rule set and
  protected-string exceptions.
- Default assistant name is **Sam**; onboarding lets any future owner rename
  the persona (see NAMING.md §3).
- `Sammy` remains the local folder name and the app-data identifier
  (`app.sammy.desktop`) until after the 0.1.0 release.

## A3. First-run onboarding (external users)

Document dump first; Calli intake second. See NAMING.md §5. Rationale: the
assistant needs enough reference material about a person before the interview
engine can ask meaningful questions.

## A4. Voice and always-on layer (future phase)

- Wake word: **"Sam"** (configurable like the persona).
- Local-first speech: small realtime STT model locally; the local Gemma model
  (KoboldCpp, Tesla M40 24GB) handles everyday chatter.
- **Stress-escalation routing:** when the local model is under load or a task
  needs more capability, reach the cloud (Venice AI — no-logging, proxied).
  This extends the existing 5-mode routing in `providers.rs` with a
  load/quality trigger; it must ride the same consent + audit path as every
  other cloud crossing.
- TTS: daily voice is a neutral assistant voice — NOT the owner's clone
  (owner clarification 2026-09-15: "I don't want to talk to myself"). The
  owner's cloned voice is reserved exclusively for the legacy protocol (A6).
  Response **compression before speech**
  (1–2 spoken sentences instead of raw payloads) is an adopted pattern; the
  TTS engine itself must be local or consent-gated cloud (never silent).
  Reference blueprint reviewed 2026-09-14 (IndyDevDan-style): its Ears/wake
  word and compress-before-speech patterns were adopted as design input; its
  ElevenLabs dependency and flat SQLite corpus were rejected as inconsistent
  with the local-first and vault-corpus architecture.

## A5. Short-term memory pattern

Session/scratchpad state (tool outputs, pending tasks, corrections) is
**application state**, human-and-agent readable, and must stay separate from
PKC durable knowledge — consistent with the master doc's storage boundary (§19–20).
A scratchpad-style store may be added for the agent layer later; it must never
write to the corpus without the Memory Review workflow.

## A6. Legacy protocol (long-term)

When the owner dies: identification required, then the assistant continues,
including the owner's cloned voice, grounded in the PKC. Architecture notes:

- Implement as a **third key-wrapping path** on the existing crypto spine
  (passphrase + recovery today; a sealed "legacy grant" envelope tomorrow)
  unlocking only specified vaults under a specified condition.
- It is a **permission profile, not a mode switch**: which vaults, which
  behaviors, rate limits, what it will and won't answer, and how it presents
  itself are all owner-set knobs.
- **Sensitive-topic indirection ("poetry-pointer rule")**: for topics the owner
  has marked sensitive, the assistant never improvises or paraphrases — it
  points to the curated artifact the owner chose. Encode as a provenance class
  and a grounding-policy rule (Calli `grounding_policy.dart` is the natural
  home), motivated by a real drafting incident where a generated letter
  overstepped exactly this boundary.
- Writing "in the owner's voice" must remain corpus-grounded and auditable:
  generated letters cite which corpus material shaped them.
