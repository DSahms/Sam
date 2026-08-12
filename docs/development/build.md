# Build

[Setup](setup.md) · [Release process](release-process.md)

Open PowerShell and use `Set-Location` to enter the Sammy repository root before
running these commands. The root contains `package.json` and `Cargo.toml`.

```powershell
npm run build          # TypeScript project build + Vite production assets
cargo build --workspace
npm run tauri:build    # release executable + MSI + NSIS
```

The first command checks and bundles the web interface. The second builds Rust
without installers. The final command combines both layers and packages Windows
installers; it may take several minutes on the first run.

Tauri runs `npm run build` as its `beforeBuildCommand`. Output appears in
`dist/`, `target/release/sammy.exe`, and `target/release/bundle/{msi,nsis}`.
Release linking can emit non-fatal `LNK4099` messages for missing vendored OpenSSL
debug PDBs; a nonzero command exit or missing bundles is the failure signal.

A successful frontend build ends with `built in ...`. A successful Tauri build
prints `Finished 2 bundles` followed by the MSI and NSIS paths.
