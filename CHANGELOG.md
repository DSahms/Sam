# Changelog

This project follows a Keep a Changelog-style structure. No published stable
release exists yet; entries describe verified repository milestones only.

## [Unreleased]

### Added

- Stage 5-b design: intake registry + the Leeds five-level probe chain
  (five invariants; Mother Leeds persona origin recorded per Dave).
- Stage 5-b-1: pure-Dart probe chain engine — Leeds levels, persona guard,
  deterministic question banks, candidate minting (18 checks).
- Stage 5-b-2: intake registry — files are truth, sqlite is speed; three
  write paths, dave-only gate, gate_log untouchable by rebuild (16 checks);
  PKC_LAYOUT_SPEC r2.1 superset dialect with unknown-field preservation.
- Stage 5-b-3 (pure-Dart core): probe session controller — model wording in
  Calli's voice with persona pre-flight (discard -> retry once -> static
  bank), wording aimed at the level being escalated TO, mint-once landing
  via a sink interface the Bench bridges to the registry; five-level pips
  state, harvest preview, and the registry integration landing.
- Stage 5-b-3 (Bench UI): probe-chain screen — mode + topic chooser, five
  level pips (filled / struck / hollow), provenance chips (model / retry /
  bank / re-ask), skip-with-reason, harvest panel noting the gate is 5-b-4.
  Working default per design §10: registry contradictions surface at
  harvest only (live in-session surfacing deferred until Dave rules).

### Fixed

- Stage 5-b-3 hotfix: ship `RegistrySessionSink` — the Bench's `main.dart`
  referenced it but the class itself was never committed. The pure-Dart
  sandbox harness used its own bridge, so the gap never surfaced there and
  only appeared as a Windows compile error (`RegistrySessionSink isn't
  defined`). The bridge now lives in the package as a pass-through
  (controller -> sink -> registry; no defaults, no swallowed errors), is
  exported from the barrel, and is pinned by three new `flutter test`
  checks: full-chain landing writes session + record files with gate_log
  untouched, re-landing never duplicates records (mint-once), and registry
  failures propagate instead of vanishing.
- Bench runtime hotfix: three independent Windows-only failures that made
  the interview loop on "What was that like?" — (1) `ApiConfig._envModel`
  called `dotenv.env` unguarded; flutter_dotenv throws
  `NotInitializedError` when no `.env` was loaded (the Bench never loads
  one), so every opening/follow-up died BEFORE contacting the local model
  and the engine's hardcoded fallback answered instead. The getter is now
  guarded exactly like `_env`: no `.env` means defaults, M40 gets contacted.
  (2) Prompt assets were loaded under bare keys
  (`assets/prompts/...`) which never exist inside a compiled app — package
  assets bundle under `packages/calli_archiviste/...`. All seven load sites
  (engine x2, compressor, narrative, long-form compiler x3) now use the
  package-qualified key, so the real Calli persona prompts ship instead of
  the placeholder fallbacks. (3) The probe-chain tab called the registry
  factory unguarded in `initState`, so with sqlite missing the whole tab
  threw `Bad state: registry unavailable` in a widget-build exception
  storm; the factory call is now wrapped and renders the existing error
  panel with the reason, while the interview tab keeps working.
  `sqlite3.dll` (public domain, sqlite.org 3.53.4 x64) ships in the apply
  bundle and goes beside the Bench's `pubspec.yaml`.

### Documentation

- Rebuilt the GitHub landing page and professional documentation hierarchy.
- Added user, concepts, architecture, security, operations, reference,
  troubleshooting, testing, release, and maintainer guidance.
- Applied the global beginner-to-maintainer documentation standard with explicit
  prerequisite verification, shell/working-directory guidance, success states,
  terminology, uninstall behavior, and diagnosis-first troubleshooting.

### Validation

- 2026-09-16: 48/48 checks green on Dave's Windows Bench machine
  (`flutter test` in calli-archiviste) at a0d5e9b — first Windows
  verification of the 5-b-3 controller suite; the Bench build error found
  the same day is fixed by the RegistrySessionSink hotfix above (+3
  checks, 51 total).
- 2026-09-16: 34/34 checks green on the Linux sandbox harness AND on Dave's
  Windows Bench machine (`flutter test`, sqlite3.dll beside pubspec.yaml) at
  commit bf2bc6f — first full cross-platform verification of Stage 5-b.
- Two Windows-exposed bugs found by the first Windows run, fixed same-day:
  sqlite3 library search (26f6cad), separator-safe json_path (bf2bc6f).

## [0.1.0-rc] - 2026-08-12

### Added

- Encrypted independent vaults, recovery, inactivity/session locking, and audit.
- Provider-independent chat with real KoboldCpp and Venice-compatible adapters.
- Structured Personal Knowledge Corpus, reviewed memory, provenance, and corpus interchange.
- TXT/Markdown/JSON/CSV/PDF/DOCX/image ingestion with local OCR adapter.
- Persistent per-vault vector retrieval and rebuild.
- Encrypted backup/restore UI with preview and explicit confirmation.
- Windows MSI and NSIS production bundles.

### Validation

- 213 Rust and 12 frontend tests passed at the release checkpoint.
- NSIS install, launch, exit, and relaunch verified on the development machine.

[Unreleased]: ROADMAP.md
[0.1.0-rc]: STATE.md
