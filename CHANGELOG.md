# Changelog

This project follows a Keep a Changelog-style structure. No published stable
release exists yet; entries describe verified repository milestones only.

## [Unreleased]

### Documentation

- Rebuilt the GitHub landing page and professional documentation hierarchy.
- Added user, concepts, architecture, security, operations, reference,
  troubleshooting, testing, release, and maintainer guidance.

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
