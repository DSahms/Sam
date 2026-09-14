# Calli Archiviste Migration Manifest

## Source Location
- **Repository:** `D:\dev\StoryKeeper-Local-Writer`
- **Git Remote:** None configured (detached state)
- **Current Branch:** `main`
- **Latest Commit:** `2d9fb87` - "Show per-chapter question count on the Run Interview dashboard."

## Current Git Status
- **Status:** Modified/Untracked
- **Modified Files:**
  - `assets/prompts/follow_up_system.txt`
  - `lib/services/interview_engine.dart`
  - `linux/flutter/generated_plugins.cmake`
  - `test/interview_opening_test.dart`
  - `test/live_childhood_restore_check.dart`
  - `test/pkc_local_generation_test.dart`
  - `windows/flutter/generated_plugins.cmake`
- **Untracked Files:**
  - `test/verify_live_sessions_test.dart`

## Current Branch Analysis
- Branch `main` is ahead of StoryKeeper's `restore/april-25-interview` branch
- Contains enhanced interview features: per-chapter question count, staged long-form narrative compiler, live Run Interview dashboard with chapter milestones
- Has `AutoInterviewRunner`, `BookInterviewRunner`, `RunInterviewTracker` services
- Has `interview_engine.dart` with `InterviewEngine` class managing interview flow

## Important Directories and Files
- `lib/services/interview_engine.dart` - Core interview engine managing question flow
- `lib/models/run_interview_dashboard_state.dart` - Dashboard state management
- `lib/models/interview_session.dart` - Interview session model
- `lib/providers/interview_provider.dart` - Provider for interview UI state
- `lib/providers/session_provider.dart` - Session management with InterviewEngine dependency
- `lib/providers/narrative_provider.dart` - Narrative generation from interview sessions
- `lib/services/auto_interview_runner.dart` - Auto-interview runner with stage management
- `lib/services/book_interview_runner.dart` - Book interview runner with per-chapter boundaries
- `lib/services/chapter_progression.dart` - Chapter/session progression rules
- `lib/services/context_compressor.dart` - Context summarization for interview sessions
- `lib/services/gemma_local_writer_service.dart` - Gemma local writer service
- `lib/services/long_form_narrative_compiler.dart` - Staged compiler for long interview sessions
- `lib/services/pkc_interview_bridge.dart` - PKC interview bridge interface
- `lib/screens/interview_screen.dart` - Main interview screen
- `lib/screens/chapter_select_screen.dart` - Chapter selection screen
- `lib/screens/chapter_preview_screen.dart` - Chapter preview screen
- `lib/screens/family_tree_screen.dart` - Family tree integration
- `assets/prompts/` - Interview system prompts (context_summary, interviewer, follow_up, etc.)

## Functionality That Appears to Belong in Sam
- **Interview engine architecture** (`interview_engine.dart`) - Core question generation and flow management
- **Auto-interview runner** (`auto_interview_runner.dart`) - Automated interview progression
- **Book interview runner** (`book_interview_runner.dart`) - Per-chapter interview with boundaries
- **Chapter progression rules** (`chapter_progression.dart`) - Bounded per-chapter question counts
- **Narrative services** (`narrative_service.dart`, `long_form_narrative_compiler.dart`) - Converting interview sessions to memoir prose
- **PKC interview bridge** (`pkc_interview_bridge.dart`) - Local PKC interview integration
- **Interview session models** (`session.dart`, `run_interview_dashboard_state.dart`) - State management
- **Provider architecture** (`interview_provider.dart`, `session_provider.dart`) - State management integration
- **Prompt system** (`assets/prompts/`) - Interview question prompts
- **Context compressor** (`context_compressor.dart`) - Interview session summarization

## Functionality That Should NOT Be Migrated
- **StoryKeeper-specific branding** - Any StoryKeeper-specific references or identifiers
- **Duplicate interview engine implementations** - Determine authoritative implementation during migration
- **Flutter/UI components** - The interview engine logic can be migrated, but Flutter widgets/screens should be evaluated separately
- **Android/iOS platform specifics** - Native platform code
- **Evaluation/test files** - Test infrastructure specific to StoryKeeper-Local-Writer

## Dependencies on External Paths
- **PKC_ROOT:** `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus` (configured in `pkc_config.dart`)
- **StoryKeeper dependency:** `package:storykeeper/services/interview_engine.dart` (imported in `session_provider.dart`)
- **Gemma model integration:** References to local Gemma model for interview generation

## Duplicated/Shared Components
- **Interview engine** appears in both StoryKeeper and StoryKeeper-Local-Writer as parallel implementations
- **PKC bridge** has identical implementation in both StoryKeeper and StoryKeeper-Local-Writer
- **Narrative compilation** services are unique to StoryKeeper-Local-Writer
- **Chapter progression rules** are unique to StoryKeeper-Local-Writer

## Risks and Conflicts
- **Duplicate interview engine:** Both StoryKeeper and StoryKeeper-Local-Writer have their own interview engine implementations. The authoritative one must be determined during migration.
- **PKC bridge divergence:** Both repositories have PKC bridge implementations that may have diverged
- **State model differences:** `run_interview_dashboard_state.dart` and related models may differ between the two repositories
- **Prompt system differences:** Interview prompts may have been customized differently in each repository

## Recommended Migration Order
1. **Extract PKC bridge** from StoryKeeper-Local-Writer into `pkc/` directory (authoritative PKC bridge)
2. **Extract interview engine** from StoryKeeper-Local-Writer into `calli-archiviste/` (becomes Calli Archiviste's core)
3. **Extract narrative services** from StoryKeeper-Local-Writer into `calli-archiviste/` or `shared/`
4. **Extract chapter progression rules** into `shared/` or `calli-archiviste/`
5. **Extract context compressor** into `shared/`
6. **Migrate interview prompts** into `calli-archiviste/assets/prompts/`
7. **Migrate session models** into `calli-archiviste/models/`
8. **Migrate provider architecture** into `calli-archiviste/providers/`
9. **Update StoryKeeper-Local-Writer** to reference the new authoritative implementations (post-migration)
10. **Decommission duplicate implementations** in StoryKeeper after verification

## Migration Decisions Requiring Human Input
- Which interview engine implementation becomes authoritative (StoryKeeper vs StoryKeeper-Local-Writer)?
- Should the PKC bridge from StoryKeeper-Local-Writer replace or supplement the existing PKC integration in Sammy?
- How should the assistant name/configurability be handled in the migrated interview engine?
- What portions of the StoryKeeper-Local-Writer Flutter UI should be migrated vs rebuilt?
- Should `gemma_local_writer_service.dart` be migrated or replaced with Sam's LLM integration?