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

/// Back up the active vault to a user-selected path. Requires the passphrase
/// and a recovery code (the package is openable with either).
#[tauri::command]
pub fn vault_backup(
    state: tauri::State<AppState>,
    vault_id: String,
    passphrase: String,
    recovery_code: String,
    out_path: String,
) -> AppResult<()> {
    let id = VaultId::parse(&vault_id)?;
    let recovery = RecoveryCode::parse(&recovery_code).map_err(|_| AppError::Crypto)?;
    state.with_registry(|reg| {
        reg.backup(
            id,
            passphrase.as_bytes(),
            &recovery,
            std::path::Path::new(&out_path),
        )
    })??;
    Ok(())
}

/// Restore a backup package using the passphrase. The restored vault becomes
/// available in the vault list (it is not auto-unlocked).
#[tauri::command]
pub fn vault_restore_passphrase(
    state: tauri::State<AppState>,
    in_path: String,
    passphrase: String,
) -> AppResult<String> {
    let id = state.with_registry(|reg| {
        reg.restore_from_passphrase(std::path::Path::new(&in_path), passphrase.as_bytes())
    })??;
    Ok(id.to_string())
}

/// Restore a backup package using the recovery code.
#[tauri::command]
pub fn vault_restore_recovery(
    state: tauri::State<AppState>,
    in_path: String,
    recovery_code: String,
) -> AppResult<String> {
    let code = RecoveryCode::parse(&recovery_code).map_err(|_| AppError::Crypto)?;
    let id = state.with_registry(|reg| {
        reg.restore_from_recovery(std::path::Path::new(&in_path), &code)
    })??;
    Ok(id.to_string())
}

/// Read a backup's header for restore preview (vault name, id, created date).
#[tauri::command]
pub fn vault_backup_preview(in_path: String) -> AppResult<serde_json::Value> {
    let h = crate::backup::read_header(std::path::Path::new(&in_path))?;
    Ok(serde_json::json!({
        "format_version": h.format_version,
        "vault_id": h.vault_id,
        "vault_name": h.vault_name,
        "created_at": h.created_at,
    }))
}

// -----------------------------------------------------------------------------
// Conversation + chat commands
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&crate::conversation::Conversation> for ConversationSummary {
    fn from(c: &crate::conversation::Conversation) -> Self {
        Self {
            conversation_id: c.conversation_id.clone(),
            title: c.title.clone(),
            created_at: c.created_at.clone(),
            updated_at: c.updated_at.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageSummary {
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub seq: i64,
}

/// List conversations in the active vault.
#[tauri::command]
pub fn conversation_list(
    state: tauri::State<AppState>,
) -> AppResult<Vec<ConversationSummary>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let list = crate::conversation::list(&conn)?;
        Ok(list.iter().map(ConversationSummary::from).collect())
    })?
}

/// Create a new conversation.
#[tauri::command]
pub fn conversation_create(
    state: tauri::State<AppState>,
    title: Option<String>,
) -> AppResult<String> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let id = crate::conversation::create(&conn, title.as_deref())?;
        Ok(id.to_string())
    })?
}

/// Load messages of a conversation.
#[tauri::command]
pub fn conversation_messages(
    state: tauri::State<AppState>,
    conversation_id: String,
) -> AppResult<Vec<MessageSummary>> {
    let cid = crate::ids::ConversationId::parse(&conversation_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        let msgs = crate::conversation::messages(&conn, cid)?;
        Ok(msgs
            .iter()
            .map(|m| MessageSummary {
                message_id: m.message_id.clone(),
                role: m.role.clone(),
                content: m.content.clone(),
                seq: m.seq,
            })
            .collect())
    })?
}

/// Result of a chat send: either the assistant response, or a consent request
/// that the UI must confirm before re-calling with `confirmed = true`.
#[derive(Debug, Serialize)]
pub struct ChatSendResult {
    pub content: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub crossed_to_cloud: bool,
    /// Present when a cloud crossing needs owner consent.
    pub consent_required: Option<CloudConsentView>,
    /// Sanitized prompt-section summaries (lengths, not bodies).
    pub prompt_summary: Vec<crate::identity::PromptSectionSummary>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CloudConsentView {
    pub provider_id: String,
    pub provider_display_name: String,
    pub model: String,
    pub routing_mode: String,
}

/// Send a chat turn. If a cloud crossing is required, returns `consent_required`
/// instead of a response; the frontend then re-calls with `confirmed = true`.
/// Uses the deterministic mock provider for now (real provider wiring is a
/// follow-up slice that adds provider configuration to settings).
#[tauri::command]
pub fn chat_send(
    state: tauri::State<AppState>,
    conversation_id: String,
    text: String,
    routing: crate::providers::RoutingMode,
    model: String,
    confirmed: bool,
) -> AppResult<ChatSendResult> {
    let cid = crate::ids::ConversationId::parse(&conversation_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        let roster = crate::chat::ProviderRoster::mock_only();
        // The mock is local, so for local_only/prefer_local/ask_before_crossing
        // it is used directly. Cloud-only/prefer-cloud require consent since the
        // mock is classified local — we simulate the consent path by checking
        // routing here without a cloud provider present.
        let turn = crate::chat::ChatTurn {
            conversation_id: cid,
            user_text: text,
            routing,
            max_tokens: 512,
            model,
        };
        let allow = confirmed;
        let consent: crate::chat::ConsentFn = Box::new(move |_| allow);
        match crate::chat::run_turn(&conn, &roster, &[true], &[], &turn, &consent) {
            Ok(r) => Ok(ChatSendResult {
                content: Some(r.response.content.clone()),
                provider: Some(r.response.provider.clone()),
                model: Some(r.response.model.clone()),
                crossed_to_cloud: r.response.crossed_to_cloud,
                consent_required: r.consent_requested.map(|c| CloudConsentView {
                    provider_id: c.provider_id,
                    provider_display_name: c.provider_display_name,
                    model: c.model,
                    routing_mode: c.routing_mode,
                }),
                prompt_summary: r.prompt_summary,
            }),
            Err(AppError::Config(msg)) if msg.contains("denied by owner") => {
                Err(AppError::Config(msg))
            }
            Err(e) => Err(e),
        }
    })?
}

// -----------------------------------------------------------------------------
// Identity commands
// -----------------------------------------------------------------------------

/// Get the active vault's companion identity (or the default if unset).
#[tauri::command]
pub fn identity_get(
    state: tauri::State<AppState>,
) -> AppResult<crate::identity::CompanionIdentity> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::identity::store::load_or_default(&conn)
    })?
}

/// Save the companion identity for the active vault.
#[tauri::command]
pub fn identity_save(
    state: tauri::State<AppState>,
    identity: crate::identity::CompanionIdentity,
    edit_summary: String,
) -> AppResult<()> {
    state.with_active(|v| {
        let mut id = identity;
        id.record_edit(edit_summary);
        let conn = v.lock_conn();
        crate::identity::store::save(&conn, &id)
    })?
}

// -----------------------------------------------------------------------------
// Knowledge commands
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct KnowledgeRecordView {
    pub record_id: String,
    pub record_type: String,
    pub canonical_text: String,
    pub status: String,
    pub review_state: String,
    pub sensitivity: String,
    pub confidence: f64,
    pub domain_tags: Vec<String>,
    pub source_ids: Vec<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub contradiction_set: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&crate::knowledge::KnowledgeRecord> for KnowledgeRecordView {
    fn from(r: &crate::knowledge::KnowledgeRecord) -> Self {
        Self {
            record_id: r.record_id.clone(),
            record_type: r.record_type.clone(),
            canonical_text: r.canonical_text.clone(),
            status: r.status.clone(),
            review_state: r.review_state.clone(),
            sensitivity: r.sensitivity.clone(),
            confidence: r.confidence,
            domain_tags: r.domain_tags.clone(),
            source_ids: r.source_ids.clone(),
            supersedes: r.supersedes.clone(),
            superseded_by: r.superseded_by.clone(),
            contradiction_set: r.contradiction_set.clone(),
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        }
    }
}

/// List all knowledge records (What I Know management view).
#[tauri::command]
pub fn knowledge_list(
    state: tauri::State<AppState>,
) -> AppResult<Vec<KnowledgeRecordView>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let all = crate::knowledge::list_all(&conn)?;
        Ok(all.iter().map(KnowledgeRecordView::from).collect())
    })?
}

/// Add a manually-entered approved fact.
#[tauri::command]
pub fn knowledge_add_fact(
    state: tauri::State<AppState>,
    text: String,
) -> AppResult<String> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let id = crate::knowledge::create(
            &conn,
            &v.vault_id.to_string(),
            &crate::knowledge::NewRecord::approved_fact(text),
        )?;
        Ok(id.to_string())
    })?
}

/// Approve a candidate record.
#[tauri::command]
pub fn knowledge_approve(
    state: tauri::State<AppState>,
    record_id: String,
) -> AppResult<()> {
    let id = crate::ids::RecordId::parse(&record_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::knowledge::approve(&conn, id, "owner")
    })?
}

/// Reject a candidate record.
#[tauri::command]
pub fn knowledge_reject(
    state: tauri::State<AppState>,
    record_id: String,
) -> AppResult<()> {
    let id = crate::ids::RecordId::parse(&record_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::knowledge::reject(&conn, id, "owner")
    })?
}

/// Tombstone a record.
#[tauri::command]
pub fn knowledge_tombstone(
    state: tauri::State<AppState>,
    record_id: String,
) -> AppResult<()> {
    let id = crate::ids::RecordId::parse(&record_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::knowledge::tombstone(&conn, id)
    })?
}

/// Correct (supersede) a record with new text.
#[tauri::command]
pub fn knowledge_correct(
    state: tauri::State<AppState>,
    record_id: String,
    new_text: String,
) -> AppResult<String> {
    let id = crate::ids::RecordId::parse(&record_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        let new_id = crate::knowledge::correct(
            &conn,
            &v.vault_id.to_string(),
            id,
            new_text,
            "owner",
        )?;
        Ok(new_id.to_string())
    })?
}

/// Lexical search over current knowledge.
#[tauri::command]
pub fn knowledge_search(
    state: tauri::State<AppState>,
    query: String,
    limit: Option<u32>,
) -> AppResult<Vec<String>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::knowledge::search_current(&conn, &query, limit.unwrap_or(20))
    })?
}

// -----------------------------------------------------------------------------
// Source commands (Phase 4)
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SourceView {
    pub source_id: String,
    pub kind: String,
    pub name: String,
    pub checksum: String,
    pub imported_at: String,
    pub status: String,
    pub bytes_len: u64,
}

impl From<&crate::sources::SourceRecord> for SourceView {
    fn from(s: &crate::sources::SourceRecord) -> Self {
        Self {
            source_id: s.source_id.clone(),
            kind: s.kind.clone(),
            name: s.name.clone(),
            checksum: s.checksum.clone(),
            imported_at: s.imported_at.clone(),
            status: s.status.clone(),
            bytes_len: s.bytes_len,
        }
    }
}

/// Import a source file. The path comes from the Tauri file dialog (owner-
/// selected; no arbitrary scanning — directive §27). The file is encrypted at
/// rest with the vault DEK.
#[tauri::command]
pub fn source_import(
    state: tauri::State<AppState>,
    file_path: String,
) -> AppResult<String> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let id =
            crate::sources::import(&conn, v.dek(), std::path::Path::new(&file_path))?;
        Ok(id.to_string())
    })?
}

/// List all sources in the active vault.
#[tauri::command]
pub fn source_list(state: tauri::State<AppState>) -> AppResult<Vec<SourceView>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let all = crate::sources::list(&conn)?;
        Ok(all.iter().map(SourceView::from).collect())
    })?
}

/// Get the extracted text of a source.
#[tauri::command]
pub fn source_extracted(
    state: tauri::State<AppState>,
    source_id: String,
) -> AppResult<String> {
    let id = crate::ids::SourceId::parse(&source_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::sources::extracted_text(&conn, id)
    })?
}

/// Delete (soft-delete) a source.
#[tauri::command]
pub fn source_delete(state: tauri::State<AppState>, source_id: String) -> AppResult<()> {
    let id = crate::ids::SourceId::parse(&source_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::sources::delete(&conn, id)
    })?
}

/// Lexical search over source extracted text.
#[tauri::command]
pub fn source_search(
    state: tauri::State<AppState>,
    query: String,
    limit: Option<u32>,
) -> AppResult<Vec<String>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::sources::search(&conn, &query, limit.unwrap_or(20))
    })?
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
