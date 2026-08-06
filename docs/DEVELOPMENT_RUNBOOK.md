# Development Runbook

How to build and run Sammy locally on Windows 11. Sammy is a Tauri 2 app:
a Rust backend (`src-tauri/`) compiled to a native binary, a React +
TypeScript frontend (`src/`) bundled by Vite and rendered in WebView2, with
a bundled SQLCipher providing encrypted storage.

## Prerequisites (Windows 11)

- **Rust, MSVC toolchain.** Install via `rustup` and select the
  `stable-x86_64-pc-windows-msvc` target. The MSVC C++ build tools (Visual
  Studio Build Tools or full Visual Studio with the "Desktop development
  with C++" workload) are required because the Rust linker and the bundled
  SQLCipher build use MSVC.
- **Node.js 22 (LTS).** Use the official installer or `nvm-windows`. Confirm
  with `node --version` (expect `v22.x`).
- **WebView2 Runtime.** Present on Windows 11 by default. Tauri 2 uses
  WebView2 as the system webview; no separate browser install is needed.
- **Git Bash** (or any POSIX shell) to run the `make` targets. The
  Makefile is written for Git Bash on Windows.

There is **no** need to install SQLite or SQLCipher system-wide. Sammy
links a **bundled** SQLCipher (`rusqlite` with the bundled SQLite + the
SQLCipher extension compiled in), so the build is self-contained and the
runtime never depends on a system SQLite DLL.

## First-time setup

```bash
# from the repo root
npm install          # installs frontend deps and @tauri-apps/cli
```

`npm install` also resolves the Rust dependencies on the first build; it
does not compile them. The first `tauri:dev` or `tauri:build` invocation
will compile the entire Rust dependency graph, which can take several
minutes.

## Day-to-day commands

The Makefile wraps these so they are identical across developer machines.

```bash
make install    # npm install
make dev        # run the app in dev mode (hot-reload frontend + cargo)
make build      # production build -> MSI / NSIS installer
make check      # the CI gate: fmt + clippy + eslint + prettier + tsc + tests
make fmt        # cargo fmt + prettier --write
make lint       # cargo fmt --check + clippy + tsc + eslint + prettier --check
make test       # cargo test --workspace + npm run test
make clean      # cargo clean + remove dist and node_modules
```

Equivalent raw commands:

```bash
npm run tauri:dev     # dev
npm run tauri:build   # production bundle
npm run test          # frontend tests (vitest)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
npm run typecheck
npm run lint
npm run format:check
```

## What `make check` runs (the CI gate)

`make check` is the single command that defines "the tree is healthy". It
runs, in order:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `npm run typecheck` (`tsc --noEmit`)
4. `npm run lint` (`eslint . --max-warnings=0`)
5. `npm run format:check` (prettier)
6. `cargo test --workspace`
7. `npm run test` (vitest)

A PR that does not pass `make check` is not mergeable.

## Dev vs production builds

- `npm run tauri:dev` builds the Rust binary in debug mode, starts Vite in
  dev mode with HMR, and points the Tauri window at the Vite dev server.
  Source maps are on; the binary is unsigned and unoptimized.
- `npm run tauri:build` builds the Rust binary in release mode, builds the
  frontend with `tsc -b && vite build`, and produces Windows installers
  (MSI and NSIS by default) in `src-tauri/target/release/bundle/`.

## Code signing

Sammy does **not yet** possess a code-signing certificate. As a result,
production builds are **unsigned** dev packages. Windows SmartScreen and
Defender will warn on first run. This is recorded as a release blocker in
`EXTERNAL_BLOCKERS.md` and must be resolved before any public release; it
is not a problem for local development.

## Repository layout

```
Sammy/
  src/                  React + TypeScript frontend
    components/         reusable React components
    views/              top-level screens
    styles/             CSS
    test/               frontend test setup and helpers
    App.tsx
    main.tsx
  src-tauri/
    src/                Rust backend, one module per boundary
      app.rs            app entry / Tauri command registration
      vault.rs          vault create / unlock / lock / key handling
      crypto.rs         Argon2id, key wrapping, AES helpers
      corpus.rs         Layer 1 store over SQLCipher
      sources.rs        Layer 0 encrypted source files
      knowledge.rs      knowledge records
      retrieval.rs      11-step retrieval pipeline
      memory.rs         memory-candidate workflow
      permissions.rs    permission table + tool-action gates
      providers.rs      provider trait + adapters + routing
      tools.rs          tool registry + declarations
      backup.rs         backup / restore
      conversation.rs   conversation transcript store
      citations.rs      citation assembly
      identity.rs       per-vault identity
      audit.rs          per-vault audit log
      settings.rs       owner settings
      ids.rs            stable ID generation
      error.rs          typed errors
      lib.rs / main.rs / bin/
  docs/                 this documentation set
  schemas/              JSON / SQL schema definitions referenced by docs
  Makefile              top-level task runner
  package.json          frontend deps + scripts
  Cargo.toml            workspace + backend deps
  rustfmt.toml          rust formatting config
  clippy.toml           clippy config
  tsconfig*.json        TypeScript config (app + node)
  vite.config.ts        Vite config
  vitest.config.ts      frontend test config
  eslint.config.js      lint config
  index.html            Vite entry HTML
```

Each Rust file in `src-tauri/src/` corresponds to one boundary described in
this documentation set; new boundaries get a new module, not edits smeared
across existing ones.

## Common snags

- **First build is slow.** The bundled SQLCipher and the full Tauri stack
  compile a lot of code; expect minutes, not seconds, on a clean tree.
- **Missing C++ build tools.** If the build fails in `build.rs` of a
  SQLite/SQLCipher crate, the MSVC C++ tools are not installed.
- **Wrong Node version.** Node 22 is required; older Node versions will
  fail on Vite 6 or the Tauri CLI.
- **Bundled, not system.** If a developer finds themselves installing
  system SQLite or SQLCipher, something is wrong — Sammy is intentionally
  bundled to avoid version drift and to keep the runtime self-contained.
