//! Sammy — encryption-first, local-first personal AI consigliere.
//!
//! Crate layout follows the architectural boundaries in the project directive
//! (§7). Each module owns one capability. Security-sensitive state lives only
//! in Rust and is never handed to the frontend in plaintext form.
//!
//! Module status:
//! - `crypto`    — Phase 1 cryptographic primitives (Argon2id, AES-GCM, KDF).
//! - `vault`     — Phase 1 vault create / unlock / lock / multi-vault.
//! - `audit`     — Phase 1 append-only audit-event foundation.
//! - `identity`, `conversation`, `providers`, `knowledge`, `corpus`,
//!   `sources`, `retrieval`, `citations`, `memory`, `permissions`, `tools`,
//!   `backup`, `settings` — stubs, filled in by later phases.
#![forbid(unsafe_code)]

pub mod audit;
pub mod backup;
pub mod citations;
pub mod conversation;
pub mod corpus;
pub mod crypto;
pub mod db;
pub mod identity;
pub mod knowledge;
pub mod lock;
pub mod memory;
pub mod permissions;
pub mod providers;
pub mod retrieval;
pub mod settings;
pub mod sources;
pub mod tools;
pub mod vault;

pub mod app;
pub mod error;
pub mod ids;

pub use app::app_data_dir;
pub use error::{AppError, AppResult};

use tauri::Manager;

// -----------------------------------------------------------------------------
// Tauri command surface
// -----------------------------------------------------------------------------

/// Returns the application's name and version for display in the UI.
#[tauri::command]
fn app_meta(state: tauri::State<app::AppState>) -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "vault_unlocked": state.is_unlocked(),
    })
}

/// Health check used by frontend smoke tests. Never touches vault state.
#[tauri::command]
fn ping() -> &'static str {
    "sammy-ok"
}

// -----------------------------------------------------------------------------
// Entry point
// -----------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app::AppState::default())
        .invoke_handler(tauri::generate_handler![
            ping,
            app_meta,
            app::vault_create,
            app::vault_list,
            app::vault_unlock,
            app::vault_unlock_recovery,
            app::vault_lock,
            app::vault_status,
            app::touch_activity,
            app::lock_policy_get,
            app::lock_policy_set,
        ])
        .setup(|app| {
            // Periodic inactivity-lock checker: a native thread ticks every 30
            // seconds and locks the vault if the configured inactivity threshold
            // has elapsed. Manual-only policy makes this a no-op. This runs for
            // the lifetime of the app process.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let state = handle.state::<app::AppState>();
                if let Ok(true) = state.check_inactivity_lock() {
                    log::info!("vault locked due to inactivity");
                }
            });
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Sammy application");
}
