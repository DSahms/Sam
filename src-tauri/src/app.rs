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

/// Recover a vault and replace its forgotten passphrase in one operation.
#[tauri::command]
pub fn vault_recover_change_passphrase(
    state: tauri::State<AppState>,
    vault_id: String,
    recovery_code: String,
    new_passphrase: String,
) -> AppResult<VaultSummary> {
    let id = VaultId::parse(&vault_id)?;
    let code = RecoveryCode::parse(&recovery_code).map_err(|_| AppError::Crypto)?;
    let vault = state.with_registry(|reg| {
        reg.recover_and_change_passphrase(id, &code, new_passphrase.as_bytes())
    })??;
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
    confirmed: bool,
) -> AppResult<String> {
    require_restore_confirmation(confirmed)?;
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
    confirmed: bool,
) -> AppResult<String> {
    require_restore_confirmation(confirmed)?;
    let code = RecoveryCode::parse(&recovery_code).map_err(|_| AppError::Crypto)?;
    let id = state.with_registry(|reg| {
        reg.restore_from_recovery(std::path::Path::new(&in_path), &code)
    })??;
    Ok(id.to_string())
}

fn require_restore_confirmation(confirmed: bool) -> AppResult<()> {
    if confirmed {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "restore requires explicit owner confirmation".into(),
        ))
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pkc: Option<crate::external_pkc::PkcTurnView>,
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
                pkc: crate::external_pkc::load_provenance(&conn, &m.message_id),
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
    /// Body-free PKC turn view (used/state/hashes only).
    pub pkc: crate::external_pkc::PkcTurnView,
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
/// Send a chat turn through the normal Chat UI path. The provider roster is
/// built from the vault's provider configuration: when KoboldCpp is configured
/// and enabled, it is preferred; the deterministic mock is always a fallback.
/// If routing requires a cloud crossing, returns `consent_required` instead of
/// a response; the frontend then re-calls with `confirmed = true`.
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
        // Load provider configuration and build the roster.
        let config = crate::settings::load(&conn)?;
        let venice_key = crate::settings::load_venice_api_key(&conn, v.dek())?;
        let roster = crate::chat::ProviderRoster::from_config(
            &config,
            venice_key.map(|key| key.to_string()),
        );
        let resolved_model = crate::chat::ProviderRoster::resolve_model(&config, &model);

        // Compute reachability for each local provider. The mock (last entry)
        // is always reachable. Real providers (e.g. KoboldCpp at index 0) are
        // reachable if test_connection succeeds.
        let mut local_reachable = Vec::with_capacity(roster.local.len());
        for p in &roster.local {
            let reachable = if p.info().id == "mock" {
                true
            } else {
                p.test_connection().is_ok()
            };
            local_reachable.push(reachable);
        }
        // Configuration presence is enough to make a cloud provider eligible.
        // Do not probe the network here: no cloud request may occur before the
        // owner sees and approves the crossing for this turn.
        let cloud_reachable = vec![true; roster.cloud.len()];

        let turn = crate::chat::ChatTurn {
            conversation_id: cid,
            user_text: text,
            routing,
            max_tokens: 512,
            model: resolved_model,
        };
        let allow = confirmed;
        let consent: crate::chat::ConsentFn = Box::new(move |_| allow);
        match crate::chat::run_turn(
            &conn,
            &roster,
            &local_reachable,
            &cloud_reachable,
            &turn,
            &consent,
        ) {
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
                pkc: r.pkc,
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
            &crate::knowledge::NewRecord::approved_fact(text.clone()),
        )?;
        let embedding = crate::retrieval::StubEmbeddingProvider;
        let vector = crate::retrieval::EmbeddingProvider::embed(&embedding, &text)?;
        let mut index = crate::retrieval::SqliteVectorIndex::new(&conn)?;
        crate::retrieval::VectorIndex::add(&mut index, &id.to_string(), vector)?;
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
        crate::knowledge::tombstone(&conn, id)?;
        let mut index = crate::retrieval::SqliteVectorIndex::new(&conn)?;
        crate::retrieval::VectorIndex::remove(&mut index, &id.to_string())
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
            new_text.clone(),
            "owner",
        )?;
        let mut index = crate::retrieval::SqliteVectorIndex::new(&conn)?;
        crate::retrieval::VectorIndex::remove(&mut index, &id.to_string())?;
        let embedding = crate::retrieval::StubEmbeddingProvider;
        let vector = crate::retrieval::EmbeddingProvider::embed(&embedding, &new_text)?;
        crate::retrieval::VectorIndex::add(&mut index, &new_id.to_string(), vector)?;
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

/// Rebuild the derived vector index from current canonical knowledge.
#[tauri::command]
pub fn vector_index_rebuild(state: tauri::State<AppState>) -> AppResult<usize> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::retrieval::rebuild_vector_index(
            &conn,
            &crate::retrieval::StubEmbeddingProvider,
        )
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

// -----------------------------------------------------------------------------
// Corpus commands (Phase 6)
// -----------------------------------------------------------------------------

/// Export the active vault's knowledge records to a corpus package file.
#[tauri::command]
pub fn corpus_export(state: tauri::State<AppState>, out_path: String) -> AppResult<()> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let pkg = crate::corpus::export_full(&conn, &v.vault_id.to_string(), &v.name)?;
        let bytes = crate::corpus::package_to_json(&pkg)?;
        std::fs::write(std::path::Path::new(&out_path), bytes)?;
        Ok(())
    })?
}

/// Import a corpus package file into the active vault (idempotent upsert).
#[tauri::command]
pub fn corpus_import(
    state: tauri::State<AppState>,
    in_path: String,
) -> AppResult<String> {
    state.with_active(|v| {
        let bytes = std::fs::read(std::path::Path::new(&in_path))?;
        let pkg = crate::corpus::package_from_json(&bytes)?;
        let conn = v.lock_conn();
        let outcome = crate::corpus::import(&conn, &v.vault_id.to_string(), &pkg)?;
        crate::retrieval::rebuild_vector_index(
            &conn,
            &crate::retrieval::StubEmbeddingProvider,
        )?;
        Ok(serde_json::to_string(&outcome)?)
    })?
}

// -----------------------------------------------------------------------------
// Memory candidate commands (Phase 7)
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MemoryCandidateView {
    pub candidate_id: String,
    pub proposed_text: String,
    pub record_type: String,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub reason: String,
    pub confidence: f64,
    pub sensitivity: String,
    pub suggested_domain: String,
    pub provider_used: String,
    pub crossed_to_cloud: bool,
    pub created_at: String,
    pub state: String,
}

impl From<&crate::memory::MemoryCandidate> for MemoryCandidateView {
    fn from(m: &crate::memory::MemoryCandidate) -> Self {
        Self {
            candidate_id: m.candidate_id.clone(),
            proposed_text: m.proposed_text.clone(),
            record_type: m.record_type.clone(),
            source_conversation_id: m.source_conversation_id.clone(),
            source_message_id: m.source_message_id.clone(),
            reason: m.reason.clone(),
            confidence: m.confidence,
            sensitivity: m.sensitivity.clone(),
            suggested_domain: m.suggested_domain.clone(),
            provider_used: m.provider_used.clone(),
            crossed_to_cloud: m.crossed_to_cloud,
            created_at: m.created_at.clone(),
            state: m.state.clone(),
        }
    }
}

/// List memory candidates, optionally filtered by state.
#[tauri::command]
pub fn memory_list(
    state: tauri::State<AppState>,
    state_filter: Option<String>,
) -> AppResult<Vec<MemoryCandidateView>> {
    let filter = state_filter
        .as_deref()
        .and_then(crate::memory::CandidateState::parse);
    state.with_active(|v| {
        let conn = v.lock_conn();
        let list = crate::memory::list(&conn, filter)?;
        Ok(list.iter().map(MemoryCandidateView::from).collect())
    })?
}

/// Approve a candidate (optionally with edited text), promoting it to an
/// approved knowledge record.
#[tauri::command]
pub fn memory_approve(
    state: tauri::State<AppState>,
    candidate_id: String,
    edited_text: Option<String>,
) -> AppResult<String> {
    let id = crate::ids::MemoryCandidateId::parse(&candidate_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        let kid = crate::memory::approve(
            &conn,
            &v.vault_id.to_string(),
            id,
            edited_text.as_deref(),
        )?;
        crate::retrieval::rebuild_vector_index(
            &conn,
            &crate::retrieval::StubEmbeddingProvider,
        )?;
        Ok(kid)
    })?
}

/// Reject a candidate.
#[tauri::command]
pub fn memory_reject(
    state: tauri::State<AppState>,
    candidate_id: String,
) -> AppResult<()> {
    let id = crate::ids::MemoryCandidateId::parse(&candidate_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::memory::reject(&conn, id)
    })?
}

/// Defer a candidate.
#[tauri::command]
pub fn memory_defer(
    state: tauri::State<AppState>,
    candidate_id: String,
) -> AppResult<()> {
    let id = crate::ids::MemoryCandidateId::parse(&candidate_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::memory::defer(&conn, id)
    })?
}

/// Mark a candidate temporary.
#[tauri::command]
pub fn memory_mark_temporary(
    state: tauri::State<AppState>,
    candidate_id: String,
) -> AppResult<()> {
    let id = crate::ids::MemoryCandidateId::parse(&candidate_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::memory::mark_temporary(&conn, id)
    })?
}

/// Delete a candidate.
#[tauri::command]
pub fn memory_delete(
    state: tauri::State<AppState>,
    candidate_id: String,
) -> AppResult<()> {
    let id = crate::ids::MemoryCandidateId::parse(&candidate_id)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::memory::delete(&conn, id)
    })?
}

// -----------------------------------------------------------------------------
// Permission + tool commands (Phase 8)
// -----------------------------------------------------------------------------

/// List the tool registry (static declarations for action previews).
#[tauri::command]
pub fn tool_registry() -> Vec<crate::permissions::ToolDeclaration> {
    crate::tools::registry()
}

/// List active (non-revoked, non-expired) permission grants.
#[tauri::command]
pub fn permission_list_active(
    state: tauri::State<AppState>,
) -> AppResult<Vec<crate::permissions::PermissionGrant>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::permissions::list_active(&conn)
    })?
}

/// Grant a permission for a tool.
#[tauri::command]
pub fn permission_grant(
    state: tauri::State<AppState>,
    tool_id: String,
    mode: crate::permissions::PermissionMode,
    scope: String,
    expires_at: Option<String>,
) -> AppResult<String> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::permissions::grant(&conn, &tool_id, mode, &scope, expires_at.as_deref())
    })?
}

/// Revoke all grants for a tool. Immediate.
#[tauri::command]
pub fn permission_revoke(
    state: tauri::State<AppState>,
    tool_id: String,
) -> AppResult<()> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::permissions::revoke(&conn, &tool_id)
    })?
}

// -----------------------------------------------------------------------------
// Audit commands (Phase 9: Privacy & Audit UI)
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AuditEventView {
    pub seq: i64,
    pub event_id: String,
    pub occurred_at: String,
    pub actor: Option<String>,
    pub category: String,
    pub action: String,
    pub detail_json: serde_json::Value,
}

/// List recent audit events (most-recent-first).
#[tauri::command]
pub fn audit_list(
    state: tauri::State<AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<AuditEventView>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let events = crate::audit::list(&conn, limit.unwrap_or(100))?;
        Ok(events
            .iter()
            .map(|e| AuditEventView {
                seq: e.seq,
                event_id: e.event_id.clone(),
                occurred_at: e.occurred_at.clone(),
                actor: e.actor.clone(),
                category: e.category.clone(),
                action: e.action.clone(),
                detail_json: e.detail_json.clone(),
            })
            .collect())
    })?
}

/// Count audit events by category.
#[tauri::command]
pub fn audit_counts(state: tauri::State<AppState>) -> AppResult<Vec<(String, i64)>> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::audit::count_by_category(&conn)
    })?
}

// -----------------------------------------------------------------------------
// Provider configuration commands (KoboldCpp connectivity)
// -----------------------------------------------------------------------------

/// Get the provider configuration for the active vault.
#[tauri::command]
pub fn provider_config_get(
    state: tauri::State<AppState>,
) -> AppResult<crate::settings::ProviderConfig> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        let mut config = crate::settings::load(&conn)?;
        config.venice_has_api_key = crate::settings::has_venice_api_key(&conn)?;
        Ok(config)
    })?
}

/// Save the provider configuration for the active vault.
#[tauri::command]
pub fn provider_config_save(
    state: tauri::State<AppState>,
    config: crate::settings::ProviderConfig,
) -> AppResult<()> {
    crate::external_pkc::validate_or_error(&config)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        let previous = crate::settings::load(&conn)?;
        let mut to_save = config;
        if pkc_identity_changed(&previous, &to_save) {
            to_save.pkc_last_health_state.clear();
            to_save.pkc_last_health_at.clear();
        }
        crate::settings::save(&conn, &to_save)?;
        if previous.pkc_enabled != to_save.pkc_enabled {
            let action = if to_save.pkc_enabled {
                "pkc_enabled"
            } else {
                "pkc_disabled"
            };
            crate::audit::record(
                &conn,
                crate::audit::AuditCategory::Knowledge,
                action,
                None,
                &serde_json::json!({
                    "consumer": crate::external_pkc::CONSUMER_APPLICATION,
                    "purpose": crate::external_pkc::PURPOSE,
                }),
            )?;
        } else if pkc_identity_changed(&previous, &to_save) {
            crate::audit::record(
                &conn,
                crate::audit::AuditCategory::Knowledge,
                "pkc_config_changed",
                None,
                &serde_json::json!({
                    "enabled": to_save.pkc_enabled,
                    "source_configured": !to_save.pkc_source_id.trim().is_empty(),
                }),
            )?;
        }
        Ok(())
    })?
}

fn pkc_identity_changed(
    previous: &crate::settings::ProviderConfig,
    next: &crate::settings::ProviderConfig,
) -> bool {
    previous.pkc_python_executable.trim() != next.pkc_python_executable.trim()
        || previous.pkc_bridge_script.trim() != next.pkc_bridge_script.trim()
        || previous.pkc_root.trim() != next.pkc_root.trim()
        || previous.pkc_source_id.trim() != next.pkc_source_id.trim()
}

/// Discover local PKC defaults (paths that actually exist on this machine).
#[tauri::command]
pub fn pkc_discover_defaults() -> crate::external_pkc::PkcDiscovery {
    crate::external_pkc::discover_defaults()
}

/// PKC health. `probe` actually talks to the gateway and discards corpus text.
#[tauri::command]
pub fn pkc_health_check(
    state: tauri::State<AppState>,
    config: crate::settings::ProviderConfig,
    probe: bool,
) -> AppResult<crate::external_pkc::PkcHealthReport> {
    let report = crate::external_pkc::health_check(&config, probe);
    state.with_active(|v| {
        let conn = v.lock_conn();
        if probe {
            let mut saved = crate::settings::load(&conn)?;
            saved.pkc_last_health_state = report.state.as_str().to_string();
            saved.pkc_last_health_at = chrono::Utc::now().to_rfc3339();
            crate::settings::save(&conn, &saved)?;
            crate::audit::record(
                &conn,
                crate::audit::AuditCategory::Knowledge,
                "pkc_health_check",
                None,
                &serde_json::json!({
                    "state": report.state.as_str(),
                    "probed": report.probed,
                    "authorized": report.authorized,
                    "python_ok": report.python_ok,
                    "bridge_ok": report.bridge_ok,
                    "root_ok": report.root_ok,
                    "payload_chars": report.payload_chars,
                    "payload_sha256": report.payload_sha256,
                }),
            )?;
        }
        Ok(report)
    })?
}

/// Owner opened provenance detail. Records an audit event without corpus text.
#[tauri::command]
pub fn pkc_provenance_opened(
    state: tauri::State<AppState>,
    message_id: String,
) -> AppResult<()> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::audit::record(
            &conn,
            crate::audit::AuditCategory::Knowledge,
            "pkc_provenance_opened",
            None,
            &serde_json::json!({ "message_id": message_id }),
        )?;
        Ok(())
    })?
}

/// Store or clear the Venice credential. The value is encrypted immediately
/// and is never returned by any command.
#[tauri::command]
pub fn venice_api_key_set(
    state: tauri::State<AppState>,
    api_key: String,
) -> AppResult<()> {
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::settings::save_venice_api_key(&conn, v.dek(), &api_key)
    })?
}

/// Validate the saved Venice configuration without sending a private prompt.
/// Credential-bearing traffic is restricted by HttpVeniceTransport to the
/// canonical Venice HTTPS host.
#[tauri::command]
pub fn venice_test_connection(state: tauri::State<AppState>) -> AppResult<Vec<String>> {
    let (endpoint, model, key) = state.with_active(|v| {
        let conn = v.lock_conn();
        let key = crate::settings::load_venice_api_key(&conn, v.dek())?
            .ok_or_else(|| AppError::Config("venice API key is not configured".into()))?;
        let config = crate::settings::load(&conn)?;
        Ok::<_, AppError>((config.venice_endpoint, config.venice_model, key.to_string()))
    })??;
    let provider = crate::providers::VeniceProvider::new(
        endpoint,
        model,
        key,
        Box::new(crate::providers::HttpVeniceTransport::new()),
    );
    let models = crate::providers::Provider::list_models(&provider)?;
    state.with_active(|v| {
        let conn = v.lock_conn();
        crate::audit::record(
            &conn,
            crate::audit::AuditCategory::Provider,
            "venice_connection_test",
            None,
            &serde_json::json!({"result": "success"}),
        )
    })??;
    Ok(models)
}

/// Test connectivity to a KoboldCpp endpoint. Returns the list of available
/// models. This makes a real HTTP GET to `<endpoint>/v1/models`. No vault
/// unlock is strictly required (the endpoint is non-secret), but we run it
/// through the active state for auditability.
#[tauri::command]
pub fn koboldcpp_test_connection(
    state: tauri::State<AppState>,
    endpoint: String,
) -> AppResult<Vec<String>> {
    // Record an audit event for the connection test.
    let _ = state.with_active(|v| {
        let conn = v.lock_conn();
        let _ = crate::audit::record(
            &conn,
            crate::audit::AuditCategory::Provider,
            "koboldcpp_connection_test",
            None,
            &serde_json::json!({"endpoint": endpoint}),
        );
        Ok::<(), AppError>(())
    });
    // Make the real HTTP call outside the vault lock (don't hold the DB
    // mutex during network I/O).
    let transport = crate::providers::HttpKoboldTransport::new();
    let raw = crate::providers::KoboldTransport::list_models(&transport, &endpoint)?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| AppError::Config(format!("koboldcpp bad model list: {e}")))?;
    let mut out = Vec::new();
    if let Some(arr) = v["data"].as_array() {
        for m in arr {
            if let Some(id) = m["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    Ok(out)
}

/// Send a chat request directly to a configured KoboldCpp endpoint (bypassing
/// the routing/consent layer for a direct manual test). This is local-only and
/// does not cross any privacy boundary.
#[tauri::command]
pub fn koboldcpp_chat(
    state: tauri::State<AppState>,
    endpoint: String,
    model: String,
    system_prompt: String,
    user_message: String,
    max_tokens: Option<u32>,
) -> AppResult<serde_json::Value> {
    let provider = crate::providers::KoboldCppProvider::new(
        endpoint.clone(),
        model.clone(),
        Box::new(crate::providers::HttpKoboldTransport::new()),
    );
    let req = crate::providers::ChatRequest {
        system: system_prompt,
        messages: vec![crate::providers::ProviderMessage {
            role: crate::conversation::Role::User,
            content: user_message,
        }],
        model,
        max_tokens: max_tokens.unwrap_or(256),
        routing: crate::providers::RoutingMode::LocalOnly,
    };
    let resp = crate::providers::Provider::chat(&provider, &req)?;
    // Audit the call.
    let _ = state.with_active(|v| {
        let conn = v.lock_conn();
        let _ = crate::audit::record(
            &conn,
            crate::audit::AuditCategory::Provider,
            "provider_local_call",
            None,
            &serde_json::json!({
                "provider": "koboldcpp",
                "model": resp.model,
                "direct_test": true,
            }),
        );
        Ok::<(), AppError>(())
    });
    Ok(serde_json::json!({
        "content": resp.content,
        "provider": resp.provider,
        "model": resp.model,
        "crossed_to_cloud": resp.crossed_to_cloud,
    }))
}

// -----------------------------------------------------------------------------
// App data dir
// -----------------------------------------------------------------------------

/// Resolve the per-user Sammy application data directory.
///
/// On Windows this is `%LOCALAPPDATA%\app.sammy.desktop`. Keeping runtime
/// data under the bundle identifier prevents an installer/uninstaller from
/// confusing user vaults with application binaries.
pub fn app_data_dir() -> PathBuf {
    let base = dirs_or_localappdata();
    let dir = base.join("app.sammy.desktop");
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

#[cfg(test)]
mod tests {
    use super::require_restore_confirmation;

    #[test]
    fn restore_is_rejected_without_explicit_confirmation() {
        assert!(require_restore_confirmation(false).is_err());
        assert!(require_restore_confirmation(true).is_ok());
    }
}
