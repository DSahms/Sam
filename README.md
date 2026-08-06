# Sammy

**Sammy** is an encryption-first, local-first personal AI consigliere for Windows 11 desktops.

The durable product is the owner-controlled combination of **identity, knowledge, sources, memory, permissions, conversation history, audit history, and personality configuration**. The language model is replaceable.

> Keep the soul. Change the brain.

Sammy can change AI providers without losing its identity, knowledge, memory, conversation history, or relationship with its owner.

## Status

This repository is under active development against a phased roadmap (see
`ROADMAP.md`). **Phase 0 (Foundation)** is complete and verified. Phase 1
(Cryptographic Vault Spine) is in progress. See `STATE.md` for exactly what
works today and `PROJECT_STATE.json` for machine-readable status.

Sammy is **not** finished, secure, or production-ready. Do not rely on it to
protect real secrets yet. See `THREAT_MODEL.md` for the honest threat boundary.

## What Sammy is not

- A generic chatbot wrapper.
- A coding agent.
- A therapy or medical product.
- A surveillance system.
- A cloud-only service.
- A vector database presented as a product.
- A system that silently converts AI guesses into facts.
- A system that acts externally without explicit permission.
- A system that uploads the owner's entire knowledge corpus to an AI provider.

## First production platform

- Windows 11, 64-bit x86.
- Per-user installation.
- Offline operation is the default; network is used only for an explicitly
  selected cloud AI provider or local-model endpoint.

See `docs/DEVELOPMENT_RUNBOOK.md` for build prerequisites and commands.

## Technology stack

| Layer        | Technology                                            |
| ------------ | ----------------------------------------------------- |
| Desktop shell| Tauri 2                                               |
| Backend      | Rust                                                  |
| Frontend      | React + TypeScript + Vite                             |
| Storage      | SQLite via SQLCipher (bundled, vendored OpenSSL)      |
| KDF          | Argon2id                                              |
| Symmetric    | AES-256-GCM                                           |

## Quick start (development, Windows 11)

Prerequisites: Rust (MSVC toolchain), Node.js 22, and Strawberry Perl on `PATH`
(Strawberry provides the complete Perl module set that vendored OpenSSL
needs; the Perl bundled with Git for Windows is incomplete for this).

```bash
npm install            # frontend dependencies
npm run tauri:dev      # run the full app in dev mode
make check             # fmt + clippy + eslint + prettier + tsc + tests
```

Build a production package (unsigned dev package until a code-signing
certificate is available — see `EXTERNAL_BLOCKERS.md`):

```bash
npm run tauri:build    # produces MSI / NSIS installers
```

## Repository layout

```
Sammy/
├── src/                     # React + TypeScript frontend
│   ├── components/          # Sidebar, Placeholder, navigation
│   └── views/               # 8 primary views (directive §33)
├── src-tauri/
│   ├── src/                 # Rust backend (module per architectural boundary)
│   │   ├── crypto.rs vault.rs audit.rs ...   # Phase 1+ modules
│   │   ├── ids.rs error.rs app.rs            # shared
│   │   └── bin/cli.rs                        # sammy-cli corpus validator
│   ├── capabilities/        # Tauri 2 permission capabilities
│   └── icons/
├── docs/                    # deep-dive architecture documents
├── Cargo.toml               # workspace root
└── Makefile                 # repeatable dev / lint / test / build targets
```

## Documents

- `SAMMY_NORTH_STAR.md` — durable product intent.
- `REQUIREMENTS.md` — requirements derived from the directive.
- `ARCHITECTURE.md` — system architecture.
- `ROADMAP.md` — phased build roadmap.
- `STATE.md` — what currently works (and only that).
- `DECISIONS.md` — material decisions and their rationale.
- `THREAT_MODEL.md` / `SECURITY_MODEL.md` — honest security boundary.
- `TEST_PLAN.md` — test strategy.
- `EXTERNAL_BLOCKERS.md` — unavailable external dependencies.
- `PROJECT_STATE.json` — machine-readable status.
- `docs/` — per-subsystem deep dives.
