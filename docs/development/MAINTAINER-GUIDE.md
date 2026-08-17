# Maintainer guide

[Documentation home](../README.md) · [Repository layout](repository-layout.md) · [Release process](release-process.md)

## Orient yourself

Read, in order: root `README.md`, `SAMMY_NORTH_STAR.md`, `STATE.md`,
`REQUIREMENTS.md`, `ARCHITECTURE.md`, `DECISIONS.md`, `THREAT_MODEL.md`, and this
documentation index. Then inspect `src-tauri/src/lib.rs` for registered commands,
`app.rs` for the boundary, and the relevant domain module and tests.

The implementation and passing tests are source of truth. `STATE.md` must describe
only verified behavior; `ROADMAP.md` must distinguish shipped work, external
validation, and future enhancement.

## Make a safe change

1. Inspect `git status`, staged/unstaged diffs, recent history, and applicable tests.
2. Preserve existing vault, provider, corpus, and command abstractions.
3. Keep cryptography, credentials, routing filters, and storage in Rust.
4. Add focused tests, then run cross-module and forbidden-behavior coverage.
5. Update user, architecture, reference, status, and changelog pages affected.
6. Run the complete release gate before committing a logical change.

Never reset or discard an unknown working tree, weaken encryption for
convenience, add silent cloud fallback, make vectors canonical, auto-promote
model output to memory, or log prompts/secrets. Use logical commits and never
track vaults, API keys, recovery codes, model files, installers, or caches.

For releases, follow the reusable checklist rather than relying on a successful
unit suite alone. An installer, installed-app smoke, documentation validation,
and [clean-Windows matrix](../getting-started/windows-clean-install-validation.md)
are required before calling a public Windows build finished. Signing readiness:
[Windows code signing](windows-code-signing.md). An honest external-prerequisite
record is part of completion.
