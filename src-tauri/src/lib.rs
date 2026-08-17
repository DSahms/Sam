//! Sammy — encryption-first, local-first personal AI consigliere.
//!
//! Crate layout follows the architectural boundaries in the project directive
//! (§7). Each module owns one capability. Security-sensitive state lives only
//! in Rust and is never handed to the frontend in plaintext form.
//!
//! Domain modules are production implementations: cryptography and vault
//! lifecycle, identity/conversations/providers, knowledge/corpus/sources,
//! retrieval/citations/memory, permissions/tools/audit, backup, and settings.
// `unsafe` is denied crate-wide; the only audited exception is the Windows
// session-lock FFI in `lock::is_session_locked_windows`. Use `#[allow(unsafe_code)]`
// only on that function, never elsewhere.
#![deny(unsafe_code)]

pub mod audit;
pub mod backup;
pub mod chat;
pub mod citations;
pub mod conversation;
pub mod corpus;
pub mod crypto;
pub mod db;
pub mod external_pkc;
pub mod grounding;
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
            app::vault_recover_change_passphrase,
            app::vault_lock,
            app::vault_status,
            app::touch_activity,
            app::lock_policy_get,
            app::lock_policy_set,
            app::vault_backup,
            app::vault_restore_passphrase,
            app::vault_restore_recovery,
            app::vault_backup_preview,
            app::conversation_list,
            app::conversation_create,
            app::conversation_messages,
            app::chat_send,
            app::identity_get,
            app::identity_save,
            app::knowledge_list,
            app::knowledge_add_fact,
            app::knowledge_approve,
            app::knowledge_reject,
            app::knowledge_tombstone,
            app::knowledge_correct,
            app::knowledge_search,
            app::vector_index_rebuild,
            app::source_import,
            app::source_list,
            app::source_extracted,
            app::source_delete,
            app::source_search,
            app::corpus_export,
            app::corpus_import,
            app::memory_list,
            app::memory_approve,
            app::memory_reject,
            app::memory_defer,
            app::memory_mark_temporary,
            app::memory_delete,
            app::memory_keep_from_chat,
            app::tool_registry,
            app::permission_list_active,
            app::permission_grant,
            app::permission_revoke,
            app::audit_list,
            app::audit_counts,
            app::provider_config_get,
            app::provider_config_save,
            app::pkc_discover_defaults,
            app::pkc_health_check,
            app::pkc_provenance_opened,
            app::venice_api_key_set,
            app::venice_test_connection,
            app::koboldcpp_test_connection,
            app::koboldcpp_chat,
        ])
        .setup(|app| {
            // Periodic lock checker: a native thread ticks every 15 seconds and
            // locks the vault if (a) the configured inactivity threshold has
            // elapsed, or (b) the OS session is currently locked (directive §10:
            // "always lock when the Windows session locks"). Manual-only policy
            // makes the inactivity check a no-op; the session-lock check always
            // applies. This runs for the lifetime of the app process.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(15));
                let state = handle.state::<app::AppState>();
                // OS session-lock: lock immediately regardless of policy.
                if crate::lock::is_session_locked() {
                    if state.is_unlocked() {
                        let _ = state.lock();
                        log::info!("vault locked due to OS session lock");
                    }
                    continue;
                }
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
