//! Process-wide application state held by Tauri, plus the Tauri command
//! surface for vault operations.
//!
//! This is the single owner of security-sensitive runtime state. The frontend
//! never receives a handle to the vault key, the unlocked database connection,
//! or provider credentials; it only receives results of operations performed
//! on its behalf.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::crypto::recovery::RecoveryCode;
use crate::error::{AppError, AppResult};
use crate::ids::VaultId;
use crate::lock::{InactivityWatchdog, LockPolicy};
use crate::vault::{Vault, VaultRegistry, VaultTemplate};

/// Process-wide state managed by Tauri. Holds the vault registry and at most
/// one unlocked vault session.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<AppStateInner>>,
}

struct AppStateInner {
    registry: VaultRegistry,
    /// `Some` when a vault is currently unlocked.
    active: Option<Vault>,
    /// Inactivity watchdog. Reset on user activity; a periodic ticker calls
    /// [`AppState::check_inactivity_lock`].
    watchdog: InactivityWatchdog,
}

impl Default for AppState {
    fn default() -> Self {
        let root = VaultRegistry::default_root();
        let registry = VaultRegistry::new(root).unwrap_or_else(|_| {
            let fallback = PathBuf::from(".").join("sammy-data").join("vaults");
            VaultRegistry::new(fallback).expect("fallback registry")
        });
        Self {
            inner: Arc::new(Mutex::new(AppStateInner {
                registry,
                active: None,
                watchdog: InactivityWatchdog::new(LockPolicy::default(), Instant::now()),
            })),
        }
    }
}

impl AppState {
    /// Run a closure with the registry. Returns a plain error to avoid
    /// holding the lock across awaits.
    pub fn with_registry<R>(&self, f: impl FnOnce(&VaultRegistry) -> R) -> AppResult<R> {
        let g = self
            .inner
            .lock()
            .map_err(|_| AppError::Config("state poisoned".into()))?;
        Ok(f(&g.registry))
    }

    /// Install an unlocked vault as the active session, replacing any prior.
    pub fn set_active(&self, vault: Vault) -> AppResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| AppError::Config("state poisoned".into()))?;
        g.active = Some(vault);
        Ok(())
    }

    /// Run a closure with the active unlocked vault.
    pub fn with_active<R>(&self, f: impl FnOnce(&Vault) -> R) -> AppResult<R> {
        let g = self
            .inner
            .lock()
            .map_err(|_| AppError::Config("state poisoned".into()))?;
        match &g.active {
            Some(v) => Ok(f(v)),
            None => Err(AppError::VaultLocked),
        }
    }

    /// Lock the active vault (drop the session). Also resets the inactivity
    /// timer so a freshly-unlocked vault gets a full window.
    pub fn lock(&self) -> AppResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| AppError::Config("state poisoned".into()))?;
        g.active = None;
        g.watchdog.touch(Instant::now());
        Ok(())
    }

    /// Record user activity (resets the inactivity timer). Called on any
    /// user-initiated vault command.
    pub fn touch_activity(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.watchdog.touch(Instant::now());
        }
    }

    /// Get the current inactivity-lock policy.
    pub fn lock_policy(&self) -> LockPolicy {
        self.inner
            .lock()
            .map(|g| g.watchdog.policy())
            .unwrap_or_default()
    }

    /// Set the inactivity-lock policy. Takes effect immediately for subsequent
    /// `should_lock` checks.
    pub fn set_lock_policy(&self, policy: LockPolicy) -> AppResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| AppError::Config("state poisoned".into()))?;
        g.watchdog = InactivityWatchdog::new(policy, Instant::now());
        Ok(())
    }

    /// Check whether the inactivity threshold has elapsed; if so, lock the
    /// vault. Called by a periodic ticker. Returns true if a lock occurred.
    pub fn check_inactivity_lock(&self) -> AppResult<bool> {
        let should = {
            let g = self
                .inner
                .lock()
                .map_err(|_| AppError::Config("state poisoned".into()))?;
            g.active.is_some() && g.watchdog.should_lock(Instant::now())
        };
        if should {
            self.lock()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.inner
            .lock()
            .map(|g| g.active.is_some())
            .unwrap_or(false)
    }

    /// The id of the active vault, if any.
    pub fn active_vault_id(&self) -> Option<VaultId> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.active.as_ref().map(|v| v.vault_id))
    }
}

// -----------------------------------------------------------------------------
// Tauri commands
// -----------------------------------------------------------------------------

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct VaultSummary {
    pub vault_id: String,
    pub name: String,
    pub template: VaultTemplate,
    pub created_at: String,
}

impl From<&crate::vault::VaultManifest> for VaultSummary {
    fn from(m: &crate::vault::VaultManifest) -> Self {
        Self {
            vault_id: m.vault_id.to_string(),
            name: m.name.clone(),
            template: m.template,
            created_at: m.created_at.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateVaultResult {
    pub vault: VaultSummary,
    /// The recovery code in human form. Shown exactly once; never stored.
    pub recovery_code: String,
}

/// Create a new vault. Returns the new vault summary and the recovery code
/// (which the frontend must display once and let the user save).
#[tauri::command]
pub fn vault_create(
    state: tauri::State<AppState>,
    name: String,
    template: VaultTemplate,
    passphrase: String,
) -> AppResult<CreateVaultResult> {
    if state.is_unlocked() {
        // Only one vault may be unlocked at a time in this process.
        return Err(AppError::Config(
            "another vault is currently unlocked; lock it first".into(),
        ));
    }
    let (manifest, _dek, recovery) = state
        .with_registry(|reg| reg.create(&name, template, passphrase.as_bytes()))??;
    // Note: we intentionally do NOT auto-unlock here. The caller unlocks
    // separately so the create+confirm-recovery flow can complete first.
    Ok(CreateVaultResult {
        vault: VaultSummary::from(&manifest),
        recovery_code: recovery.to_human_string(),
    })
}

/// Enumerate vaults visible on disk.
#[tauri::command]
pub fn vault_list(state: tauri::State<AppState>) -> AppResult<Vec<VaultSummary>> {
    state
        .with_registry(|reg| reg.list())?
        .map(|v| v.iter().map(VaultSummary::from).collect())
}

/// Unlock a vault with the master passphrase.
#[tauri::command]
pub fn vault_unlock(
    state: tauri::State<AppState>,
    vault_id: String,
    passphrase: String,
) -> AppResult<VaultSummary> {
    let id = VaultId::parse(&vault_id)?;
    let vault = state
        .with_registry(|reg| reg.unlock_with_passphrase(id, passphrase.as_bytes()))??;
    let summary = VaultSummary {
        vault_id: vault.vault_id.to_string(),
        name: vault.name.clone(),
        template: vault.template,
        created_at: String::new(), // not held on the session; omitted
    };
    state.set_active(vault)?;
    Ok(summary)
}

/// Unlock a vault with a recovery code.
#[tauri::command]
pub fn vault_unlock_recovery(
    state: tauri::State<AppState>,
    vault_id: String,
    recovery_code: String,
) -> AppResult<VaultSummary> {
    let id = VaultId::parse(&vault_id)?;
    let code = RecoveryCode::parse(&recovery_code).map_err(|_| AppError::Crypto)?;
    let vault = state.with_registry(|reg| reg.unlock_with_recovery(id, &code))??;
    let summary = VaultSummary {
        vault_id: vault.vault_id.to_string(),
        name: vault.name.clone(),
        template: vault.template,
        created_at: String::new(),
    };
    state.set_active(vault)?;
    Ok(summary)
}

/// Lock the active vault.
#[tauri::command]
pub fn vault_lock(state: tauri::State<AppState>) -> AppResult<()> {
    state.lock()
}

/// Whether a vault is currently unlocked, and if so its id.
#[tauri::command]
pub fn vault_status(state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({
        "unlocked": state.is_unlocked(),
        "active_vault_id": state.active_vault_id().map(|i| i.to_string()),
    })
}

/// Record user activity, resetting the inactivity timer. The frontend should
/// call this on meaningful user input.
#[tauri::command]
pub fn touch_activity(state: tauri::State<AppState>) {
    state.touch_activity();
}

/// Get the current inactivity-lock policy.
#[tauri::command]
pub fn lock_policy_get(state: tauri::State<AppState>) -> LockPolicy {
    state.lock_policy()
}

/// Set the inactivity-lock policy.
#[tauri::command]
pub fn lock_policy_set(
    state: tauri::State<AppState>,
    policy: LockPolicy,
) -> AppResult<()> {
    state.set_lock_policy(policy)
}

// Suppress unused import warning when RecoveryCode import is only used in type.
#[allow(unused_imports)]
use RecoveryCode as _RecoveryCode;

// -----------------------------------------------------------------------------
// App data dir
// -----------------------------------------------------------------------------

/// Resolve the per-user Sammy application data directory.
///
/// On Windows this is `%LOCALAPPDATA%\Sammy`. Created if missing.
pub fn app_data_dir() -> PathBuf {
    let base = dirs_or_localappdata();
    let dir = base.join("Sammy");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[cfg(target_os = "windows")]
fn dirs_or_localappdata() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local);
    }
    let home = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join("AppData").join("Local")
}

#[cfg(not(target_os = "windows"))]
fn dirs_or_localappdata() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local").join("share");
    }
    PathBuf::from(".")
}

impl Default for AppStateInner {
    fn default() -> Self {
        unreachable!("AppState::default constructs this directly")
    }
}
