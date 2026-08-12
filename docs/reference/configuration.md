# Configuration reference

[Settings](../user-guide/settings.md) · [Providers](../user-guide/providers.md)

Provider configuration is stored per vault. KoboldCpp fields include enabled,
base endpoint, and selected model. Venice fields include enabled, fixed endpoint,
selected model, and encrypted key-presence state. Routing mode is selected per
chat request from `local_only`, `cloud_only`, `prefer_local`, `prefer_cloud`, and
`ask_before_crossing`.

Vault inactivity policies are manual-only or 5, 15, 30, or 60 minutes. Windows
session locking remains active regardless of the inactivity choice.

Build configuration lives in `src-tauri/tauri.conf.json`; frontend scripts are in
`package.json`; Rust dependencies/features are in the workspace and Tauri
`Cargo.toml` files.
