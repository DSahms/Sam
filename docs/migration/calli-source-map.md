# Calli Archiviste — Source Map

**Purpose:** Authoritative mapping of every component to its donor file(s) and target location. Used by implementation agents to verify only required files are migrated.

---

## Legend

- **DONOR** = Source file in `D:\dev\StoryKeeper-Local-Writer` (the Local-Writer fork)
- **TARGET** = Destination in `Sammy/calli-archiviste/` (new Calli repo)
- **REFERENCE** = Informational only; not copied
- **EXCLUDED** = Writer-only; never migrated to Calli

---

## Core Interview Engine

| Component | Donor Path | Target Path | Status |
|-----------|------------|-------------|--------|
| Interview Engine | `lib/services/interview_engine.dart` | `lib/services/interview_engine.dart` | **MIGRATE** |
| Session Model | `lib/models/session.dart` | `lib/models/session.dart` | **MIGRATE** (rename to `IntakeSession`) |
| Message Model | `lib/models/message.dart` | `lib/models/message.dart` | **MIGRATE** |
| Photo Metadata Model | `lib/models/photo_metadata.dart` | `lib/models/photo_metadata.dart` | **MIGRATE** |
| PKC Interview Evidence Model | `lib/models/pkc_interview_evidence.dart` | `lib/models/pkc_interview_evidence.dart` | **MIGRATE** |
| Interview Provider | `lib/providers/interview_provider.dart` | `lib/providers/interview_provider.dart` | **MIGRATE** |
| Interview Screen | `lib/screens/interview_screen.dart` | `lib/screens/interview_screen.dart` | **MIGRATE** |
| Media Capture Screen | `lib/screens/media_capture_screen.dart` | `lib/screens/media_capture_screen.dart` | **MIGRATE** |
---

## PKC Bridge & Contract

| Component | Donor Path | Target Path | Status |
|-----------|------------|-------------|--------|
| PKC Interview Bridge | `lib/services/pkc_interview_bridge.dart` | `lib/services/pkc_bridge.dart` | **MIGRATE + REFACTOR** (thin DTO only) |
| Context Compressor | `lib/services/context_compressor.dart` | `lib/services/context_compressor.dart` | **MIGRATE** |
| Grounding Policy | `lib/services/grounding_policy.dart` | `lib/services/grounding_policy.dart` | **MIGRATE** |
| Chapter Progression (generic) | `lib/services/chapter_progression.dart` | `lib/services/chapter_progression.dart` | **MIGRATE** (strip memoir constants) |

---

## Storage & Media

| Component | Donor Path | Target Path | Status |
|-----------|------------|-------------|--------|
| Session Storage | `lib/services/session_storage.dart` | `lib/services/session_storage.dart` | **MIGRATE + SPLIT** (knowledge_box / ui_state_box) |
| Media Service | `lib/services/media_service.dart` | `lib/services/media_service.dart` | **MIGRATE + ADAPT** (delegate to MediaArchive) |

---

## PKC Software (Sam Side)

| Component | Donor Path | Target Path | Status |
|-----------|------------|-------------|--------|
| PKC Rust Core | `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus` | `Sammy/pkc/` | **REFERENCE** (authoritative source) |
| External PKC Shim | `src-tauri/src/external_pkc.rs` | `Sammy/pkc/` | **REFERENCE** |
| PKC Interview Adapter (Python) | `tools/pkc_interview_adapter.py` | `Sammy/pkc/` | **REFERENCE** |
| PKC Interview Gateway (Python) | `tools/pkc_interview_gateway.py` | `Sammy/pkc/` | **REFERENCE** |
---

## Explicitly EXCLUDED (Writer / Local-Writer Only)

| Component | Donor Path | Reason |
|-----------|------------|--------|
| Book Formatter | `lib/services/book_formatter.dart` | Writer output |
| Long Form Narrative Compiler | `lib/services/long_form_narrative_compiler.dart` | Writer output |
| Print Service | `lib/services/print_service.dart` | Writer output |
| Book Provider | `lib/providers/book_provider.dart` | Writer UI |
| Narrative Provider | `lib/providers/narrative_provider.dart` | Writer UI |
| Chapter Provider | `lib/providers/chapter_provider.dart` | Writer UI |
| Run Interview Tracker | `lib/services/run_interview_tracker.dart` | Local-Writer dashboard |
| Auto Interview Runner | `lib/services/auto_interview_runner.dart` | Local-Writer automation |
| Book Interview Runner | `lib/services/book_interview_runner.dart` | Local-Writer automation |
| Story Session Selector | `lib/services/story_session_selector.dart` | Writer preview |
| Session Export Service | `lib/services/session_export_service.dart` | Writer forensics |
| Gemma Local Writer Service | `lib/services/gemma_local_writer_service.dart` | Local-Writer LLM |
| Kobold Local Service | `lib/services/kobold_local_service.dart` | Local-Writer LLM |
| Claude Service | `lib/services/claude_service.dart` | Sam AI (shared) |
| Groq Service | `lib/services/groq_service.dart` | Sam AI (shared) |
| Venice Service | `lib/services/venice_service.dart` | Sam AI (shared) |
| LLM Service Interface | `lib/services/llm_service.dart` | Sam AI (shared) |
| LLM Factory | `lib/services/llm_factory.dart` | Sam AI (shared) |
| Local Writer Settings | `lib/models/local_writer_settings.dart` | Local-Writer config |
| Local Writer Settings Storage | `lib/services/local_writer_settings_storage.dart` | Local-Writer config |
| Book Config | `lib/models/book_config.dart` | Writer config |
| Run Interview Dashboard State | `lib/models/run_interview_dashboard_state.dart` | Local-Writer UI |
| Recovery Constants | `lib/config/recovery_constants.dart` | NURA product |
| Chapter Catalog | `lib/config/chapter_catalog.dart` | Writer memoir |
| Constants | `lib/config/constants.dart` | Writer memoir |
| All Narrative Prompts | `assets/prompts/narrative_*.txt` | Writer output |
| Interviewer System Prompt | `assets/prompts/interviewer_system.txt` | Writer interview |

---

## Shared Utilities (Minimal)

| Component | Donor Path | Target Path | Status |
|-----------|------------|-------------|--------|
| Utils | `lib/utils.dart` | `Sammy/shared/utils.dart` | **MIGRATE IF NEEDED** (only if cross-component) |
| Constants | `lib/constants.dart` | `Sammy/shared/constants.dart` | **MIGRATE IF NEEDED** (only if cross-component) |

---

## Original StoryKeeper (Reference Only)

| Component | Path | Status |
|-----------|------|--------|
| Original StoryKeeper App | `D:\dev\StoryKeeper` | **REFERENCE ONLY — NEVER COPY** |

---

## Verification Checklist

- [ ] Every **MIGRATE** row has a corresponding file in `Sammy/calli-archiviste/`
- [ ] No **EXCLUDED** row appears in `Sammy/calli-archiviste/`
- [ ] All `package:storykeeper/` imports removed from migrated files
- [ ] Hive boxes split: `knowledge_box` vs `ui_state_box`
- [ ] PKC bridge refactored to thin DTO interface
- [ ] Media service delegates to `MediaArchive` abstract class
- [ ] Schema version pinned in `PKCInterviewEvidence`