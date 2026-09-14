# PKC Migration Manifest

## Source Location
- **Repository:** `F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus`
- **Git Remote:** `https://github.com/DSahms/personal-knowledge-corpus.git`
- **Current Branch:** `codex/pkc-retrieval-mvp`
- **Latest Commit:** `b0234cc` - "feat: authorize Sammy as a distinct read-only PKC consumer"

## Current Git Status
- **Status:** Clean
- **Modified Files:** None
- **Untracked Files:** None

## Current Branch Analysis
- Branch `codex/pkc-retrieval-mvp` is actively developing
- Latest commit specifically authorizes Sammy as a distinct read-only PKC consumer
- Contains retrieval, evidence, and conformance testing infrastructure

## Important Directories and Files
- `tools/pkc_interview_gateway.py` - Core PKC gateway for interview-driven retrieval
- `tools/pkc_intake.py` - Knowledge intake and ingestion pipeline
- `tools/pkc_retrieval.py` - Knowledge retrieval engine
- `tools/pkc_search.py` - Search functionality
- `tools/pkc_evidence.py` - Evidence model and management
- `tools/pkc_answer_envelope.py` - Structured response envelope format
- `tools/pkc_consumer_conformance.py` - Consumer compliance testing
- `tools/pkc_build_index.py` - Index building and maintenance
- `tools/source_intake.py` - Source intake and validation
- `tools/evaluate_pkc_retrieval.py` - Retrieval evaluation
- `tools/evaluate_pkc_evidence.py` - Evidence evaluation
- `tools/export_portable_corpus.py` - Portable corpus export
- `tools/retrieval_promotion.py` - Retrieval promotion lifecycle
- `03_approved_corpus/` - Approved knowledge corpus data
- `01_raw_archive/` - Raw archived artifacts
- `04_identity_kernel/` - Identity kernel data
- `05_projects/` - Project knowledge
- `06_exports/` - Exported knowledge packages
- `00_inbox/` - Pending intake items
- `docs/` - Documentation and governance

## Functionality That Appears to Belong in Sam
- **Authoritative PKC storage** - The core knowledge corpus should live in `pkc/`
- **PKC gateway** (`pkc_interview_gateway.py`) - Interview-driven retrieval interface
- **PKC intake pipeline** (`pkc_intake.py`) - Knowledge ingestion and processing
- **PKC retrieval** (`pkc_retrieval.py`) - Knowledge search and retrieval
- **PKC evidence model** (`pkc_evidence.py`) - Evidence representation and management
- **PKC answer envelope** (`pkc_answer_envelope.py`) - Standardized response format
- **PKC consumer conformance** (`pkc_consumer_conformance.py`) - Ensures Sam meets PKC requirements
- **Portable corpus export** (`export_portable_corpus.py`) - Cross-platform knowledge exchange
- **Source intake** (`source_intake.py`) - Validation and ingestion of new knowledge sources
- **Evaluation tools** - Quality assurance for retrieval and evidence quality

## Functionality That Should NOT Be Migrated
- **Raw personal data/archive files** - Large personal documents and artifacts should remain in their existing locations or be migrated separately with consent
- **Evaluation fixtures** - Test data specific to PKC development should not be migrated into the product structure
- **00_inbox/ and draft directories** - Pending intake items and drafts are operational data, not product code
- **Personal identity kernel** - Sensitive personal data should be handled with care and may need separate migration
- **Project-specific narratives** - NURA and other project-specific content should not be migrated into Sam's core

## Dependencies on External Paths
- **External repository path:** This PKC repository is currently external to the Sam repository and must be migrated in
- **No hardcoded local paths found** in the PKC codebase itself (uses relative paths and environment configuration)
- **Python-based:** Requires Python environment for gateway execution
- **SQLite/JSON storage:** Knowledge corpus stored in structured files and potentially SQLite

## Duplicated/Shared Components
- **PKC bridge in StoryKeeper:** Both StoryKeeper and StoryKeeper-Local-Writer have their own PKC bridge implementations that call into this PKC repository
- **PKC bridge in Sammy:** `src-tauri/src/external_pkc.rs` provides read-only access to the external PKC
- **No duplicate PKC core implementation** exists in the Sam repository currently - PKC core lives entirely in the external repository
- **Shared conformance testing** between Sam and PKC consumer implementations

## Risks and Conflicts
- **External path dependency:** Sammy currently depends on the external PKC path via `SAMMY_PKC_ROOT` or similar environment configuration
- **Concurrent development:** PKC repository is actively being developed on `codex/pkc-retrieval-mvp` branch
- **Data migration risk:** Large archive of personal documents may be substantial and require careful handling
- **Conformance requirements:** Sam must satisfy PKC's consumer conformance requirements after migration
- **Interview gateway integration:** Calli Archiviste will need to integrate with the PKC interview gateway
- **Version alignment:** Must ensure Sam's PKC integration matches the PKC repository's current API contract

## Recommended Migration Order
1. **Migrate PKC core code** (`tools/` directory) into `pkc/` directory in the Sam repository
2. **Migrate PKC data directories** (`03_approved_corpus/`, `04_identity_kernel/`, `05_projects/`) into `pkc/`
3. **Migrate documentation** (`docs/`) into `pkc/docs/`
4. **Update Sammy's `external_pkc.rs`** to reference the internal PKC implementation instead of the external path
5. **Update StoryKeeper-Local-Writer's PKC bridge** to use the new internal PKC path
6. **Update StoryKeeper's PKC bridge** to use the new internal PKC path
7. **Run PKC consumer conformance tests** against Sam's integration
8. **Verify interview gateway functionality** with Calli Archiviste integration
9. **Update documentation** in the Sam repository to reference the internal PKC location
10. **Decommission external PKC dependency** after all consumers are updated

## Migration Decisions Requiring Human Input
- What portion of the personal archive data should be migrated into the Sam repository?
- How should sensitive personal data (identity kernel, raw archive) be handled during migration?
- Should the PKC repository's current branch state be frozen for migration or should it continue evolving?
- How should the PKC interview gateway be integrated with Calli Archiviste's interview engine?
- What are the exact conformance requirements for Sam as a PKC consumer?
- Should the PKC repository's GitHub remote be preserved or should it become part of the Sam repository's history?