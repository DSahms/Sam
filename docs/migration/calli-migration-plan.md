# Calli Archiviste Migration Plan

## What Calli Is

Calli Archiviste is the **Personal Knowledge Intake Engine**. Its scope is strictly bounded to:

- Accepting artifacts (photos, documents, audio, text) from the user
- Conducting structured interviews about those artifacts to gather who/what/when/where
- Capturing personal meaning, context, and provenance for each artifact
- Preserving the link between structured knowledge and the original artifact
- Creating structured knowledge packets (DTOs) from interview evidence
- Submitting those packets to the Personal Knowledge Corpus (PKC) via a thin, versioned contract
- Maintaining a local session store for in-progress interviews only

Calli is **not** a writer, publisher, formatter, or narrative assembler. It stops at structured knowledge hand-off.

---

## What Calli Is Not

| Excluded Capability | Belongs To |
|---------------------|------------|
| Book generation / formatting | Writer (Sam) |
| Long-form narrative compilation | Writer (Sam) |
| Print/PDF/EPUB output | Writer (Sam) |
| Chapter progression UI for memoir | Writer (Sam) |
| Auto-interview runners (Gemma-driven) | Writer (Sam) / Local-Writer |
| StoryKeeper session dashboard / run tracking | Writer (Sam) / Local-Writer |
| LLM provider abstraction (Claude, Groq, Venice, Kobold) | Sam (shared AI service) or Writer |
| Recovery-journey chapter constants | NURA product (separate) |
| Book configuration / cover templates | Writer (Sam) |
| Session export / forensic tooling | Writer (Sam) / Local-Writer |
| Story session selection for preview | Writer (Sam) |
| UI providers: book_provider, narrative_provider, chapter_provider | Writer (Sam) |
---

## Authoritative Source by Component

| Component | DONOR (Source) | TARGET |
|-----------|----------------|--------|
| Interview Engine | `D:\dev\StoryKeeper-Local-Writer\lib\services\interview_engine.dart` | `Sammy/calli-archiviste/lib/services/interview_engine.dart` |
| Interview Models (Session, Message, PhotoMetadata, PKCInterviewEvidence) | `D:\dev\StoryKeeper-Local-Writer\lib\models\session.dart`, `message.dart`, `photo_metadata.dart`, `pkc_interview_evidence.dart` | `Sammy/calli-archiviste/lib/models/` |
| Interview Provider (UI glue) | `D:\dev\StoryKeeper-Local-Writer\lib\providers\interview_provider.dart` | `Sammy/calli-archiviste/lib/providers/interview_provider.dart` |
| Interview Screen (artifact intake UI) | `D:\dev\StoryKeeper-Local-Writer\lib\screens\interview_screen.dart`, `media_capture_screen.dart` | `Sammy/calli-archiviste/lib/screens/` |
| PKC Bridge (thin DTO) | `D:\dev\StoryKeeper-Local-Writer\lib\services\pkc_interview_bridge.dart` | `Sammy/calli-archiviste/lib/services/pkc_bridge.dart` |
| Context Compressor | `D:\dev\StoryKeeper-Local-Writer\lib\services\context_compressor.dart` | `Sammy/calli-archiviste/lib/services/context_compressor.dart` |
| Grounding Policy | `D:\dev\StoryKeeper-Local-Writer\lib\services\grounding_policy.dart` | `Sammy/calli-archiviste/lib/services/grounding_policy.dart` |
| Chapter Progression Rules | `D:\dev\StoryKeeper-Local-Writer\lib\services\chapter_progression.dart` | `Sammy/calli-archiviste/lib/services/chapter_progression.dart` |
| Session Storage (knowledge fields only) | `D:\dev\StoryKeeper-Local-Writer\lib\services\session_storage.dart` | `Sammy/calli-archiviste/lib/services/session_storage.dart` |
| Media Service (capture + hash) | `D:\dev\StoryKeeper-Local-Writer\lib\services\media_service.dart` | `Sammy/calli-archiviste/lib/services/media_service.dart` |
| PKC Software (Rust) | `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus` | `Sammy/pkc/` |
| PKC External Shim | `D:\dev\StoryKeeper-Local-Writer\src-tauri\src\external_pkc.rs` | `Sammy/pkc/` (reference) |
| PKC Interview Adapter (Python) | `D:\dev\StoryKeeper-Local-Writer\tools\pkc_interview_adapter.py` | `Sammy/pkc/` (reference) |
| PKC Interview Gateway (Python) | `D:\dev\StoryKeeper-Local-Writer\tools\pkc_interview_gateway.py` | `Sammy/pkc/` (reference) |
---

## PKC Software vs Personal Data

| Category | Location | Git? |
|----------|----------|------|
| PKC Application Code (Rust, Python adapters) | `Sammy/pkc/` | **YES** — application code |
| PKC Interview Gateway/Adapter | `Sammy/pkc/` | **YES** — application code |
| Personal PKC Repository (user's knowledge corpus) | `~/.local/share/pkc/` or user-chosen path | **NO** — personal data, never in Git |
| Calli Interview Evidence (in-flight) | `Sammy/calli-archiviste/` local Hive | **NO** — personal data, never in Git |
| Media Archive Artifacts | `Sammy/media-archive/` user-chosen path | **NO** — personal data, never in Git |

**Rule:** Only application code goes to Git. Personal knowledge, artifacts, and PKC repositories are user data — excluded via `.gitignore`.

---

## Artifact vs Knowledge Boundary

| Artifact (Media Archive) | Knowledge (PKC) |
|--------------------------|-----------------|
| Original photo file (JPEG, HEIC, RAW) | Structured metadata: who, what, when, where, context, meaning |
| Original document (PDF, DOCX) | Extracted entities, relationships, narrative fragments |
| Original audio (WAV, MP3) | Transcript segments, speaker tags, emotional markers |
| Provenance: SHA-256 hash, capture timestamp, device info | Provenance reference: `artifact_id` linking to Media Archive |
| Stored in: `Sammy/media-archive/` (user-controlled, versioned) | Stored in: `Sammy/pkc/` (user's PKC repository) |

**Contract:** Calli writes artifacts to Media Archive → receives `artifact_id` → includes `artifact_id` in PKC DTO → submits DTO to PKC. Calli never stores artifact bytes in PKC.

---

## StoryKeeper Dependency Removal

| Dependency | Action |
|------------|--------|
| `package:storykeeper/` imports | **Remove entirely** — replace with Calli-internal models |
| `LifeChapter` enum | **Remove** — Calli has no chapter concept; chapters are Writer/UI concern |
| `ChapterCatalog`, `ChapterProgression` (memoir-specific) | **Remove** — replace with generic `InterviewPhase` if needed |
| `SessionState` (StoryKeeper-specific) | **Remove** — replace with `InterviewSessionState` (idle, interviewing, submitting, complete) |
| `MessageRole.interviewer` / `user` / `system` | **Keep** — generic roles, rename package |
| `InterviewSession` model | **Keep** — rename to `IntakeSession`, strip Writer fields |
---

## Calli → PKC Contract

**Interface:** `PKCBridge.submitEvidence(PKCInterviewEvidence evidence) -> Result<SubmissionReceipt, PKCBridgeError>`

**DTO: `PKCInterviewEvidence`**
```dart
class PKCInterviewEvidence {
  final String sessionId;
  final String artifactId;           // from Media Archive
  final Map<String, dynamic> structuredKnowledge;  // who/what/when/where/meaning
  final List<ProvenanceEntry> provenance;
  final DateTime capturedAt;
  final String schemaVersion;        // e.g., "calli.evidence.v1"
}
```

**Rules:**
- Calli knows **only** this DTO shape
- Calli does **not** format, narrate, or assemble
- PKC side owns schema evolution; Calli pins to a version
- No StoryKeeper types in the DTO
- No Local-Writer formatting logic in the bridge

---

## Media Archive Boundary

| Responsibility | Owner |
|----------------|-------|
| Artifact ingestion (copy, hash, store) | Media Archive |
| Provenance metadata (device, timestamp, GPS, EXIF) | Media Archive |
| Artifact ID allocation (content-addressable or UUID) | Media Archive |
| Artifact retrieval by ID | Media Archive |
| Versioning / deduplication | Media Archive |
| Calli integration | Calli calls `MediaArchive.saveArtifact(File, Metadata) -> ArtifactId` |

**Calli does not:**
- Manage file storage paths
- Compute hashes (delegates to Media Archive)
- Store artifact bytes in Hive or PKC

---

## Migration Boundary

| KEEP in Calli | MOVE to Sam/PKC/Media Archive | EXCLUDE (Writer-only) |
|---------------|-------------------------------|----------------------|
| `interview_engine.dart` | PKC bridge adapter (thin DTO only) → `Sammy/pkc/` | `book_formatter.dart` |
| `interview_provider.dart` | Media Archive service → `Sammy/media-archive/` | `long_form_narrative_compiler.dart` |
| `pkc_interview_bridge.dart` (refactored) | PKC persistence (Rust) → `Sammy/pkc/` | `print_service.dart` |
| `session_storage.dart` (knowledge fields) | LLM provider abstraction → `Sammy/shared/ai/` (optional) | `book_provider.dart` |
| `media_service.dart` (capture + hash → Media Archive) | Chapter progression (memoir) → Writer | `narrative_provider.dart` |
| `context_compressor.dart` | Auto-interview runners → Writer/Local-Writer | `chapter_provider.dart` |
| `grounding_policy.dart` | Recovery constants → NURA | `run_interview_tracker.dart` |
| `chapter_progression.dart` (generic only) | Story session selector → Writer | `auto_interview_runner.dart` |
| `session.dart`, `message.dart`, `photo_metadata.dart`, `pkc_interview_evidence.dart` | Session export/forensics → Writer | `book_interview_runner.dart` |
| Intake UI screens (`interview_screen.dart`, `media_capture_screen.dart`) | Local-Writer settings → Writer | `gemma_local_writer_service.dart` |
| | Kobold/Claude/Groq/Venice services → Sam AI | `kobold_local_service.dart` |
| | | `claude_service.dart` |
| | | `groq_service.dart` |
| | | `venice_service.dart` |
| | | `local_writer_settings*.dart` |
| | | `recovery_constants.dart` |
| | | `book_config.dart` |
| | | Narrative feature wiring (providers/compilers) — NOTE: narrative prompt TEXT was handed over 2026-09-15 and is vendored in `calli-archiviste/assets/prompts/` for the future memoir-assembly path |

---

## Migration Anti-Patterns

**Explicitly prohibited:**

1. **Copying the entire Local Writer application** — Calli is a subset, not a clone
2. **Copying the original StoryKeeper application** (`D:\dev\StoryKeeper`) — reference only, never copy
3. **Copying personal PKC/archive data into Git** — personal data never versioned
4. **Blindly copying the existing PKC bridge** — current bridge has Writer formatting logic; must be thinned to DTO-only
5. **Creating a duplicate interview engine** — extract the existing one, don't rewrite
6. **Putting everything into `shared/`** — no cross-component contracts exist yet; shared stays minimal
7. **Global search/replace** (e.g., `storykeeper` → `calli`) — surgical edits only
8. **Preserving StoryKeeper identity in Calli** — package name, constants, enums, chapter concepts must be removed
9. **Migrating UI state into PKC** — only knowledge fields migrate
10. **Migrating artifact bytes into PKC** — artifacts go to Media Archive; PKC gets references only
11. **Assuming the current Python adapter is the final contract** — it's a donor reference; the contract is defined here
12. **Skipping the Hive box split** — without it, UI state pollutes knowledge submissions

---

## Implementation Order

1. **Create Calli target foundation** — `Sammy/calli-archiviste/` Flutter package, `pubspec.yaml`, directory structure
2. **Extract the required interview/intake engine** — copy authoritative source files (see Source Map), rename package, remove StoryKeeper imports
3. **Remove the StoryKeeper package dependency** — replace all `package:storykeeper/` imports with `package:calli_archiviste/`; delete unused enums/constants
4. **Define the Calli → PKC DTO/interface** — create `PKCInterviewEvidence`, `PKCBridge` interface, `SubmissionReceipt`; pin schema version
5. **Separate application/session state from knowledge** — split Hive into `knowledge_box` and `ui_state_box`; migrate only knowledge fields
6. **Define Media Archive interface and artifact references** — create `MediaArchive` abstract class in Calli; stub implementation; integrate `media_service.dart` to call it
7. **Integrate Calli with PKC** — implement `PKCBridge` using the Python adapter (or Rust FFI); verify DTO round-trip
8. **Integrate Calli with Sam** — expose Calli as a library Sam can call; define Sam → Calli API
9. **Add migration tests and verification** — unit tests for DTO serialization, bridge contract, Hive split, Media Archive contract
10. **Only after successful verification consider removing/renaming legacy naming** — final cleanup of any remaining `storykeeper` references

---

## Acceptance Criteria

| Criterion | Verification |
|-----------|--------------|
| Calli builds as standalone Flutter package | `flutter pub get && flutter analyze` passes in `Sammy/calli-archiviste/` |
| No `package:storykeeper/` imports remain | `grep -r "package:storykeeper" Sammy/calli-archiviste/` returns empty |
| PKC DTO serializes/deserializes without StoryKeeper types | Unit test: `PKCInterviewEvidence.fromJson(jsonEncode(dto)) == dto` |
| Hive split: `knowledge_box` contains only PKC-bound fields | Inspection: no UI state keys in `knowledge_box` |
| Media Archive contract compiles and stubs work | `MediaArchive.saveArtifact()` returns `ArtifactId` in test |
| Bridge submits DTO to PKC adapter without formatting | Integration test: adapter receives DTO, no narrative fields present |
| Calli can be added as dependency to Sam | `Sammy/apps/sam/pubspec.yaml` adds `calli_archiviste:` path dependency |
| No personal data in Git | `git status` shows no `.hive`, `.pkc`, artifact files |
| Anti-pattern checklist passed | Manual review against Migration Anti-Patterns list |

---

Implementation status: ENGINE EXTRACTED (2026-09-15) — the authoritative donor
InterviewEngine and its dependency world (chapter catalog, family/photo models,
LLM service stack, PKC bridge config, InterviewSession storage) are extracted
into `calli-archiviste/` with imports remapped; the Calli scaffold is preserved
alongside (IntakeSession/IntakePhase/ArtifactMetadata + IntakeStorage; the
evidence model was healed from a corrupted copy and now carries Calli's own
wire identity). Donor prompt assets (`assets/prompts/*.txt`, 6 files) were
handed over 2026-09-15 and are now vendored + declared in pubspec.yaml: the
engine and compressor load the REAL prompts (interviewer, follow_up,
context_summary); the three narrative_* prompts are staged as assets for the
memoir-assembly path (no Dart consumer yet — storage and model config already
exist). Fallback placeholder strings remain only as a degraded-mode safety
net that logs loudly on asset-load failure. Still pending: host LLM wiring
and `flutter analyze` on the owner's machine.
Plan status: PROMPTS LANDED (2026-09-15) — awaiting owner-side flutter analyze

---

## Storage Boundary

| Hive Box | Contents | Migrates to PKC? |
|----------|----------|------------------|
| `knowledge_box` | `IntakeSession` (structured fields only), `PKCInterviewEvidence` (pending), `PhotoMetadata` (hash + artifact_id only) | **YES** |
| `ui_state_box` | Last screen, theme, window size, interview scroll position, auto-interview settings | **NO** — local ephemeral |

**Migration rule:** Only `knowledge_box` is submitted to PKC. `ui_state_box` is deleted on uninstall or cleared per-session.

---

## Source Map Addendum (recovered rows)

The following rows were misplaced at the end of the Storage Boundary table by
an earlier editing pass and are restored here as a proper source map:

| Component | Source | Target |
|-----------|--------|--------|
| Hive boxes: `sessions`, `messages`, `media` | Local-Writer | **Split** → `knowledge_box` (PKC-bound) + `ui_state_box` (local only) |
| Constants: `constants.dart`, `chapter_catalog.dart`, `recovery_constants.dart` | Local-Writer | **Remove** — not Calli's concern |
| Prompts: `interviewer_system.txt`, `narrative_*.txt` | Local-Writer | **Vendored** in `calli-archiviste/assets/prompts/` (2026-09-15) — interview + context prompts are consumed by the extracted engine; narrative prompts staged for memoir assembly |
| Sam Application | `D:\dev\StoryKeeper-Local-Writer` (selected services) | `apps/sam/` |
| Media Archive | Not yet implemented | `media-archive/` |
| Shared Utilities | `D:\dev\StoryKeeper-Local-Writer\lib\utils.dart` (minimal) | `shared/` (only if cross-component contract exists) |