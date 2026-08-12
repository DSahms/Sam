# Build

[Setup](setup.md) · [Release process](release-process.md)

```powershell
npm run build          # TypeScript project build + Vite production assets
cargo build --workspace
npm run tauri:build    # release executable + MSI + NSIS
```

Tauri runs `npm run build` as its `beforeBuildCommand`. Output appears in
`dist/`, `target/release/sammy.exe`, and `target/release/bundle/{msi,nsis}`.
Release linking can emit non-fatal `LNK4099` messages for missing vendored OpenSSL
debug PDBs; a nonzero command exit or missing bundles is the failure signal.
