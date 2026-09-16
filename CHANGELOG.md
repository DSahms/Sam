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

### Documentation

- Rebuilt the GitHub landing page and professional documentation hierarchy.
- Added user, concepts, architecture, security, operations, reference,
  troubleshooting, testing, release, and maintainer guidance.
- Applied the global beginner-to-maintainer documentation standard with explicit
  prerequisite verification, shell/working-directory guidance, success states,
  terminology, uninstall behavior, and diagnosis-first troubleshooting.

### Validation

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
