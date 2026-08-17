# Sammy — State

This file describes **only what currently works**, with evidence. It is updated
every checkpoint. If something is not here, it does not work yet.

Last updated: 2026-08-17

**External PKC** is not Sammy's vault corpus. Canonical PKC:
`F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus`.
Sammy may query it read-only as consumer `sammy` / purpose `personal_consigliere`
when the owner enables it in Settings (default **off**). Daily use is the
Settings card + normal Chat, not the diagnostic script.

Verified daily-use slice:
- Settings UI with discovery, validation, test-connection, and health states
- Retrieval only after a **local** route is committed (cloud turns never query)
- Lightweight retrieval judgment (personal/history vs greetings/math/generic)
- Sanitized model context; body-free provenance in Chat
- No automatic durable memory writes
- Audit events without corpus text
- Failures degrade; chat remains usable

Diagnostic (not the UX): `python tools/exercise_pkc_readonly_path.py --mode health`

## Works now

### Phase 0 — Foundation (verified)
- Tauri 2 + Rust + React + TS + Vite. `cargo build --workspace` and `vite build`
  succeed. SQLCipher + OpenSSL bundled. All §7 module boundaries as Rust modules.
- Quality gate: `cargo fmt`, `cargo clippy -D warnings`, eslint, prettier, tsc strict.
- `sammy-cli` binary with `corpus validate` subcommand.

### Phase 1 — Cryptographic vault spine (verified)
- `crypto`: SecretKey (zeroizing, constant-time eq), CSPRNG, Argon2id, AES-256-GCM,
  key wrap/unwrap, VaultKeyMaterial (passphrase + recovery unwrap), recovery codes.
- `vault`: per-vault dir (vault.json + vault.db); create/unlock/lock/list/delete;
  multi-vault isolation; SQLCipher encrypted-at-rest (tested); atomic writes.
- `db`: versioned, forward-only migrations with transactional rollback (v1–v7).
- `audit`: append-only audit_events API (record/list/count_by_category).
- `lock`: inactivity watchdog (5/15/30/60 min, manual); Windows session-lock
  detection (desktop-name polling via GetUserObjectInformationW); background thread.
- `backup`: SAMMYBK1 encrypted package format; create/restore (passphrase or
  recovery); integrity verification; path-traversal protection; staging + atomic
  rename.
- Tauri commands wired; functional Vaults UI (create/unlock/lock, recovery-code
  display, inactivity-policy picker).

### Phase 2 — Identity, chat, providers (verified)
- `identity`: structured CompanionIdentity (§12); PromptAssembly (8 sanitized
  sections); inspection_view (lengths only); per-vault store.
- `conversation`: conversations + messages model; create/append/list/messages.
- `providers`: Provider trait; MockProvider (deterministic), KoboldCppProvider
  (local, transport-abstracted), VeniceProvider (cloud, key-gated);
  resolve_routing (5 modes, default ask_before_crossing).
- `chat` runtime: orchestrates identity → routing (committed first) → prompt →
  consent → provider → audit; cloud-crossing consent flow; denial → no
  transmission (audited). External PKC retrieval runs only on committed local
  turns and never on cloud fallback.
- Tauri commands wired; functional Chat UI (conversations, send, routing picker,
  cloud-crossing consent banner with Approve/Deny, personal-knowledge provenance).

### Phase 3 — Structured knowledge (verified)
- `knowledge`: 18 record types, 7 states, sensitivity, provenance, contradictions,
  corrections (supersession), tombstones; FTS5 search over current records only.
- Tauri commands wired; What I Know UI (add fact, search, status filters,
  approve/reject/correct/tombstone, contradiction/supersession badges).

### Phase 4 — Secure source ingestion (verified)
- `sources`: SourceKind classification; Extractor trait + Text/Markdown/JSON/CSV
  extractors; AES-256-GCM encrypted storage; import/list/extracted/delete/search.
- FTS5 query sanitization (phrase-quoting) in both sources and knowledge search.
- Tauri commands wired; Sources UI (file-picker import, search, preview, delete).

### Phase 5 — Retrieval and citations (verified)
- `citations`: CitationLocation, TrustClassification, ClaimLabel, ClaimBlock,
  AnnotatedAnswer with consistency check (no fabricated citation ids).
- `retrieval`: EmbeddingProvider + StubEmbeddingProvider; VectorIndex +
  InMemoryVectorIndex (cosine); retrieve() 11-step pipeline; assemble_answer().
- The vector index is rebuildable and is NOT the canonical corpus.

### Phase 6 — Corpus import/export (verified)
- `corpus`: CorpusManifest + CorpusRecord + CorpusPackage; export_full (read-only);
  validate_package (version, checksum, unique ids); import (idempotent upsert,
  tombstones); JSON round-trip. JSON schemas in schemas/.
- CLI: `sammy-cli corpus validate <package.json>`. Tauri commands wired.

### Phase 7 — Auditable memory (verified)
- `memory`: MemoryCandidate with full provenance; CandidateState
  (pending/approved/rejected/deferred/temporary); propose/approve (promotes to
  knowledge record)/reject/defer/mark_temporary/delete/list.
- Explicit **Add to Memory Review** from a PKC-grounded chat turn creates a
  pending candidate with body-free PKC provenance. Retrieval never creates
  candidates or durable memory.
- PKC-derived approvals are `local_only`, keep fact/testimony/inference class,
  and do not rewrite PKC.
- Conversation content never becomes durable memory without review (§26).
- Tauri commands wired.

### Phase 8 — Permission architecture (verified)
- `permissions`: RiskLevel, Reversibility, ToolDeclaration, ToolRequest,
  PermissionMode; grant/check/revoke/list_active; revocation immediate.
- `tools`: registry of 5 tools (source_search, corpus_lookup, draft_generation,
  planning, mock_external); NO external actions execute.
- Tauri commands wired.

### Phase 9 — Product validation (in progress)
- **Forbidden-behavior test suite (§37):** 19 automated tests proving Sammy does
  NOT permit private access before unlock, wrong-passphrase success, cross-vault
  access, silent cloud fallback, tombstoned/deleted record retrieval, automatic
  memory approval, rejected memory retrieval, duplicate corpus imports, fake
  citation IDs, tool execution without permission, permanent auth from
  conversation, plaintext secrets in errors/Debug, corrupted backup restore,
  migration without rollback, mock cloud-crossing claims, or candidate records
  in current truth.
- **End-to-end workflow tests (§36):** 9 integration tests covering new-vault
  lifecycle, recovery (backup → restore on new registry via passphrase and
  recovery code), multiple vaults with isolation verification, source-grounded
  answers, corpus package round-trip with reimport-no-duplicates, memory
  workflow, provider privacy, and permissions.
- **Dependency license report:** `LICENSES.md` — 462 crates scanned, **no
  GPL/copyleft crates**; all permissive licenses.
- **Privacy & Audit UI** wired: audit summary by category + recent events table.
- **Memory Review UI** wired: candidate review queue with approve/edit/reject/
  defer/temporary/delete actions.

## Verified Phase exit criteria
- ✅ Private/chat/source access impossible before unlock.
- ✅ Wrong passwords fail safely; recovery-key restore works.
- ✅ Database content not plaintext-readable; vaults isolated.
- ✅ DB migration has rollback protection.
- ✅ Locking clears sensitive state; inactivity + session lock.
- ✅ Backup restores via passphrase or recovery; integrity verified.
- ✅ No silent cloud crossing; denial audited; no transmission.
- ✅ Provider change never erases identity/conversations.
- ✅ Facts/inference distinguishable; contradictions coexist; corrections preserve history.
- ✅ Tombstoned/deleted records excluded from retrieval.
- ✅ Corpus re-import creates no duplicates; tombstones remove availability.
- ✅ Memory requires review; rejected never in retrieval.
- ✅ No tool executes without permission; revocation immediate.
- ✅ FTS5 queries sanitized (no operator injection).

## Release-candidate additions verified 2026-08-12
- PDF and DOCX extraction plus a local Tesseract image-OCR adapter; imports are
  checksum-idempotent and retain extraction provenance.
- Real Venice HTTPS transport, encrypted per-vault API-key storage, endpoint
  restriction, model configuration, connection test, consent, and safe errors.
- Persistent per-vault SQLCipher vector index, hybrid Chat retrieval, deletion,
  restart persistence, and deterministic rebuild.
- Backup & Recovery UI with preview, both credentials, and explicit confirmation.
- Recovery-code flow replaces a forgotten passphrase without changing vault data.
- Production MSI and NSIS packages. NSIS install, launch, exit, and relaunch pass.
- 213 Rust and 12 frontend tests pass (225 total); all quality gates are green.
- Professional GitHub documentation covers installation, every product view,
  corpus concepts, architecture, security, operations, development, testing,
  release, troubleshooting, and maintainer handoff with validated local links.

## Daily-use PKC milestone verified 2026-08-16
- Owner configures PKC from Settings (discover defaults, validate, test
  connection, enable/disable). Consumer `sammy`, purpose `personal_consigliere`.
- Chat retrieves PKC only after a local route is committed. Cloud and
  local→cloud fallback never invoke the bridge and cannot carry PKC evidence.
- Retrieval judgment skips greetings, arithmetic, and generic world facts.
- Chat shows a lightweight **Used personal knowledge** provenance control
  (identifiers and sizes, not corpus bodies).
- `cargo test --lib`: 223 passed. Workspace extras also green. Vitest: 12 passed.
- Windows release build: `target\release\sammy.exe` launched successfully.
  Bundles: `target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi` and
  `target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe`.
- Diagnostic: `python tools/exercise_pkc_readonly_path.py --mode health|auth|sanitize`.

## PKC Memory Review milestone verified 2026-08-16
- PKC retrieval still writes **zero** automatic candidates and **zero** durable
  Sammy memory.
- Owner can **Add to Memory Review** from a PKC-grounded chat turn. That creates
  a pending candidate with source id, class (stored knowledge / described /
  suggestion), and hashes — not corpus bodies.
- Approve / edit / reject uses the existing Memory Review page. Only approval
  writes durable memory (`local_only`). Editing the candidate does not rewrite
  PKC. Inference stays a conclusion, not a silent fact upgrade.
- Duplicate pending keeps reuse the existing candidate. Conflicting durable
  text from the same source is surfaced, not auto-picked.
- Pending PKC-derived candidates survive restart. PKC going offline does not
  drop them.
- `cargo test --lib`: 240 passed. e2e: 9. forbidden: 19. Vitest: 15.
- Windows release rebuild: `target\release\sammy.exe`,
  `target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi`,
  `target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe`. Packaged UI exposes
  Settings PKC, Memory Review, and Chat. Vault unlock / Test connection /
  Chat keep were left for the following packaged click-through.

## Packaged Windows PKC owner click-through verified 2026-08-17
- Tested the rebuilt packaged application at
  `C:\Users\david\ZCodeProject\Sammy\target\release\sammy.exe`, using the
  existing local KoboldCpp model
  `koboldcpp/gemma-4-E4B-it-qat-UD-Q4_K_XL`.
- Used only the synthetic fixture generated by
  `tools/pkc-clickthrough-fixture/build_fixture.py`:
  - `The maple kettle is stored in the blue cupboard.`
  - `I described the porch bell as cracked.`
  - `The spare key may still be taped under the garden bench.`
  Source:
  `SRC-SHA256-74eec99bd0d69bde691bf182efe555743b7fe07b849512ba5a4440d51c58d77b`.
- Real packaged UI proved vault creation and unlock, Settings discovery,
  fixture configuration, PKC enable/save/test, and a visible **Authorized**
  state. Discovery was allowed to finish before replacing the real local
  default with the synthetic fixture.
- The controlled question `Where did I store the maple kettle? Answer using
  the stored personal knowledge.` produced the local-model answer `The maple
  kettle is stored in the blue cupboard. (Fact / stored knowledge)` with
  **Used personal knowledge** provenance.
- Retrieval alone left `Candidates (0)` and wrote no durable knowledge.
  Explicit **Add to Memory Review** created one pending candidate with the
  PKC source id and `from personal knowledge` badge; unrelated fixture body
  text was absent from its provenance.
- The pending candidate survived restart. Plain approval created durable
  local knowledge, which survived a second restart. A separate packaged run
  proved edit-before-approve (`The porch bell is cracked.` plus the original
  proposal / `owner_edited` marker) and reject (garden-bench material remained
  absent from durable knowledge); the rebuilt executable reopened and
  reverified both states.
- After disabling and saving PKC, the same personal question still received a
  normal local-model response without PKC provenance. Audit counts showed
  `pkc_retrieval_succeeded` unchanged while `provider_local_call` increased by
  one. The six-file fixture snapshot was byte-for-byte unchanged, and the
  external PKC repository remained clean at
  `b0234ccfa52096aaaa5290b2b7253f9ced136853`.
- Two real packaged blockers were fixed:
  1. Vault password fields could contain a live autofilled/injected value while
     React state was stale. Submit now reads the live input value and still
     uses the normal backend authentication path; empty unlock is rejected.
  2. OpenAI-compatible provider payloads omitted the assembled system prompt,
     so KoboldCpp never received retrieved knowledge or grounding rules. The
     system prompt is now the first `system` message for KoboldCpp and Venice.
- Validation after the fixes: Rust lib **241 passed**, e2e **9 passed**,
  forbidden **19 passed**, frontend **18 passed**; strict Clippy, ESLint, and
  TypeScript checks pass.
- Rebuilt Windows artifacts:
  `target\release\sammy.exe`,
  `target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi`, and
  `target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe`.

## Independent clean-Windows installer + code-signing readiness 2026-08-17
- Starting HEAD was `61c7bf0` (packaged PKC owner validation). Working tree was clean.
- Phase 1 forensics (before this rebuild) recorded unsigned artifacts:
  - EXE SHA-256 `766522A21BE318B68CF8B8062D898BCE5EB135A49FDC32ED6ADAC8C8A27B30B8`
  - MSI SHA-256 `84C6C4887833AD6312D99BDC4EE952EB77409708898C10183C7F442B66434DBE`
  - NSIS SHA-256 `4DB31B8F2CCE311EEF82126997E8F01EAF2FEA8C76F27627F855CB7468401C15`
- Tauri config still has no `bundle.windows` signing block. Installers remain
  NSIS current-user (`%LOCALAPPDATA%\Sammy`) and MSI per-machine
  (`C:\Program Files\Sammy`). App data stays at
  `%LOCALAPPDATA%\app.sammy.desktop`.
- Independent clean VM / Windows Sandbox / spare PC: **UNVALIDATED**. This host
  is Windows 11 Pro 10.0.26200; Sandbox is absent; no Hyper-V VMs exist. The
  daily NSIS install under `%LOCALAPPDATA%\Sammy` was not overwritten.
- Strongest isolated checks actually run:
  - MSI administrative extract (`msiexec /a`) without installing to Program Files
  - NSIS payload extract
  - Launch of extracted / rebuilt `sammy.exe` with a throwaway `LOCALAPPDATA`
    (window title `Sammy`; created `app.sammy.desktop\vaults`)
- Portability defect (failure-first, then fixed): production discovery baked
  `F:\personal-knowledge-corpus-scaffold\...` and `D:\dev\StoryKeeper\...` into
  `sammy.exe`. Those paths are gone from production code. **Find local defaults**
  still fills Python and consumer/purpose; the owner chooses folders. Optional
  env `SAMMY_PKC_ROOT` / `SAMMY_PKC_BRIDGE` can hint discovery. Vendored OpenSSL
  may still embed `OPENSSLDIR` build-prefix strings; that is not an install-time
  path lookup.
- Code signing: readiness plan only. No certificate purchased, no secrets
  generated, EXE/MSI/NSIS still `NotSigned`.
- Validation after the portability fix: Rust lib **242 passed**, e2e **9**,
  forbidden **19**, frontend **18**, windows_packaging **7**; Clippy `-D warnings`,
  ESLint, and TypeScript pass.
- Rebuilt Windows artifacts (unsigned):
  - `target\release\sammy.exe` (11,841,536 bytes)
    SHA-256 `133545CC30BE3EBB9CA33677B25C064681F40BC9C2E12026D16992A2DD7D7DEF`
  - `target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi` (5,554,176 bytes)
    SHA-256 `A4EA025D82185F8D818C35926B3E15A61F74A7956F9CFE1ED0D35085FFA6F2B0`
  - `target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe` (4,086,778 bytes)
    SHA-256 `17BCB5A99A2B00DCD758D5398B00520F550F6EE01A95851A2B152D4A4F0A66E8`

## Resume point
Installer forensics, portability checks, signing-readiness documentation, and a
human clean-Windows procedure exist. Independent clean-Windows install/uninstall
and production Authenticode signing are still outstanding. PKC-grounded chat and
Memory Review behavior are unchanged. Canonical PKC remains
`b0234ccfa52096aaaa5290b2b7253f9ced136853`.
PKC, Sammy, StoryKeeper, and the PKC Reference Client stay separate.

## External validation still required
- A spare Windows 11 PC or VM running
  `docs/getting-started/windows-clean-install-validation.md`
- Windows code-signing certificate (see `docs/development/windows-code-signing.md`)
- A live Venice API key
- Local Tesseract for image OCR

## External blockers
See `EXTERNAL_BLOCKERS.md`. None block the build or tests.

## Next work
Execute the clean-Windows validation procedure on a spare Windows 11 PC or VM
that does not contain the Sammy source tree. Do not purchase a certificate in
that milestone unless the owner separately decides to. Do not restart PKC
architecture.
