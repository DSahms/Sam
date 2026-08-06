//! Process-wide application state held by Tauri.
//!
//! This is the single owner of security-sensitive runtime state. The frontend
//! never receives a handle to the vault key, the unlocked database connection,
//! or provider credentials; it only receives results of operations performed
//! on its behalf.

use std::sync::Mutex;

use crate::ids::VaultId;

/// Phase 0 placeholder for the unlocked-vault session. Phase 1 replaces this
/// with a real `VaultSession` (DB connection + metadata), and operations check
/// this before touching any private data.
#[derive(Default)]
pub struct AppState {
    inner: Mutex<AppStateInner>,
}

#[derive(Default)]
struct AppStateInner {
    /// `Some` when a vault is currently unlocked.
    active_vault: Option<VaultId>,
}

impl AppState {
    pub fn lock(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.active_vault = None;
        }
    }

    pub fn set_active(&self, id: VaultId) {
        if let Ok(mut g) = self.inner.lock() {
            g.active_vault = Some(id);
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.inner
            .lock()
            .map(|g| g.active_vault.is_some())
            .unwrap_or(false)
    }
}
