//! Chat runtime — orchestrates identity, conversation, providers, routing,
//! and audit for a single chat turn (Phase 2).
//!
//! This is the "Agent and Conversation Runtime" layer from the architecture
//! (§7). It:
//! 1. Loads the structured identity and assembles a sanitized prompt.
//! 2. Resolves routing against reachable providers.
//! 3. Enforces privacy boundaries (local-only never crosses; cloud needs
//!    explicit consent).
//! 4. Calls the selected provider.
//! 5. Records the assistant message and an audit event (every cloud
//!    transmission is audited).
//!
//! The runtime does not know which concrete provider it's calling — it works
//! against the [`Provider`](crate::providers::Provider) trait, so the brain is
//! replaceable while the soul persists.

use std::sync::Arc;

use crate::audit::{self, AuditCategory};
use crate::conversation::{self, Role};
use crate::error::{AppError, AppResult};
use crate::identity::{self, PromptAssembly};
use crate::providers::{
    ChatRequest, ChatResponse, Provider, ProviderMessage, RoutingDecision, RoutingMode,
};

/// A consent callback used when a cloud crossing is required. Returns `true` if
/// the owner approved transmitting content to the cloud provider.
pub type ConsentFn = Box<dyn Fn(&CloudConsentRequest) -> bool + Send + Sync>;

/// Details presented to the owner when asking consent to cross to a cloud
/// provider. Contains no secrets — only what the owner needs to decide.
#[derive(Debug, Clone)]
pub struct CloudConsentRequest {
    pub provider_id: String,
    pub provider_display_name: String,
    pub model: String,
    pub routing_mode: String,
    /// Approximate prompt token count, for transparency.
    pub estimated_prompt_tokens: u32,
}

/// A configured set of providers available to the runtime. Local providers come
/// first (matching the routing index convention).
pub struct ProviderRoster {
    pub local: Vec<Arc<dyn Provider>>,
    pub cloud: Vec<Arc<dyn Provider>>,
}

impl ProviderRoster {
    /// A roster with only the deterministic mock provider (the default for an
    /// offline / fresh install).
    pub fn mock_only() -> Self {
        Self {
            local: vec![Arc::new(crate::providers::MockProvider::echo())],
            cloud: vec![],
        }
    }

    /// Build a roster from the vault's provider configuration. When KoboldCpp
    /// is configured and enabled, it is placed first in the local list so the
    /// routing resolver prefers it. The deterministic mock is always appended
    /// as a fallback so chat degrades gracefully if the real endpoint is down.
    ///
    /// Returns the roster plus a model-name resolution: if the caller passed an
    /// empty model, the configured default model is used.
    pub fn from_config(
        config: &crate::settings::ProviderConfig,
        venice_api_key: Option<String>,
    ) -> Self {
        let mut local: Vec<Arc<dyn Provider>> = Vec::new();
        if config.koboldcpp_enabled && !config.koboldcpp_endpoint.is_empty() {
            local.push(Arc::new(crate::providers::KoboldCppProvider::new(
                config.koboldcpp_endpoint.clone(),
                config.koboldcpp_model.clone(),
                Box::new(crate::providers::HttpKoboldTransport::new()),
            )));
        }
        // Always keep the mock as a fallback so chat degrades gracefully.
        local.push(Arc::new(crate::providers::MockProvider::echo()));
        let mut cloud: Vec<Arc<dyn Provider>> = Vec::new();
        if config.venice_enabled && !config.venice_endpoint.is_empty() {
            if let Some(key) = venice_api_key.filter(|key| !key.is_empty()) {
                cloud.push(Arc::new(crate::providers::VeniceProvider::new(
                    config.venice_endpoint.clone(),
                    config.venice_model.clone(),
                    key,
                    Box::new(crate::providers::HttpVeniceTransport::new()),
                )));
            }
        }
        Self { local, cloud }
    }

    /// Resolve a model name: use the requested model if non-empty, otherwise
    /// fall back to the configured default.
    pub fn resolve_model(
        config: &crate::settings::ProviderConfig,
        requested: &str,
    ) -> String {
        if requested.is_empty() {
            config.koboldcpp_model.clone()
        } else {
            requested.to_string()
        }
    }

    /// Get a provider by combined index (local first, then cloud).
    pub fn get(&self, index: usize) -> Option<&Arc<dyn Provider>> {
        if index < self.local.len() {
            self.local.get(index)
        } else {
            self.cloud.get(index - self.local.len())
        }
    }

    /// Whether each local provider is reachable (test_connection is expensive in
    /// real adapters; the caller may pre-compute and pass a cached result).
    pub fn local_reachable(&self, cached: &[bool]) -> Vec<bool> {
        (0..self.local.len())
            .map(|i| cached.get(i).copied().unwrap_or(false))
            .collect()
    }

    pub fn cloud_reachable(&self, cached: &[bool]) -> Vec<bool> {
        (0..self.cloud.len())
            .map(|i| cached.get(i).copied().unwrap_or(false))
            .collect()
    }
}

/// Inputs to a single chat turn.
pub struct ChatTurn {
    pub conversation_id: crate::ids::ConversationId,
    pub user_text: String,
    pub routing: RoutingMode,
    pub max_tokens: u32,
    pub model: String,
}

/// The result of a chat turn, with everything the UI needs to render it.
#[derive(Debug)]
pub struct ChatTurnResult {
    pub response: ChatResponse,
    pub consent_requested: Option<CloudConsentRequest>,
    pub prompt_summary: Vec<crate::identity::PromptSectionSummary>,
    pub pkc: crate::external_pkc::PkcTurnView,
}

/// Run one chat turn. `conn` is the unlocked vault's SQLCipher connection.
/// `consent` is invoked only if routing requires a cloud crossing.
pub fn run_turn(
    conn: &rusqlite::Connection,
    roster: &ProviderRoster,
    local_reachable: &[bool],
    cloud_reachable: &[bool],
    turn: &ChatTurn,
    consent: &ConsentFn,
) -> AppResult<ChatTurnResult> {
    // 1. Persist the user message immediately (durable history).
    conversation::append_message(
        conn,
        turn.conversation_id,
        Role::User,
        &turn.user_text,
    )?;

    // 2. Load identity + assemble the prompt (sanitized).
    let identity = identity::store::load_or_default(conn)?;
    let history = conversation::messages(conn, turn.conversation_id)?;
    let provider_messages: Vec<ProviderMessage> = history
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| ProviderMessage {
            role: Role::parse(&m.role).unwrap_or(Role::User),
            content: m.content.clone(),
        })
        .collect();

    // Resolve routing before any PKC retrieval so a later cloud decision cannot
    // inherit private evidence, and so cloud turns never query PKC at all.
    let decision = crate::providers::resolve_routing(
        turn.routing,
        roster.local.len(),
        roster.cloud.len(),
        &roster.local_reachable(local_reachable),
        &roster.cloud_reachable(cloud_reachable),
    );
    let local_committed = matches!(
        decision,
        RoutingDecision::Use { provider_index } if provider_index < roster.local.len()
    );

    let sensitivity = if local_committed {
        crate::retrieval::SensitivityFilter::All
    } else {
        crate::retrieval::SensitivityFilter::ExcludeLocalOnly
    };
    let vector_index = crate::retrieval::SqliteVectorIndex::new(conn)?;
    let embedding = crate::retrieval::StubEmbeddingProvider;
    let hits = crate::retrieval::retrieve(
        conn,
        &crate::retrieval::RetrievalRequest {
            query: turn.user_text.clone(),
            sensitivity,
            ..Default::default()
        },
        Some(&embedding),
        Some(&vector_index),
    )?;
    let mut retrieved: Vec<String> = hits
        .iter()
        .map(|hit| format!("[record:{}] {}", hit.record_id, hit.snippet))
        .collect();

    let pkc_outcome = if local_committed {
        match crate::settings::load(conn) {
            Ok(cfg) => crate::external_pkc::consult_for_local_turn(&cfg, &turn.user_text),
            Err(_) => crate::external_pkc::consult_for_local_turn(
                &crate::settings::ProviderConfig::default(),
                &turn.user_text,
            ),
        }
    } else {
        crate::external_pkc::skipped_cloud_bound()
    };
    let _ = crate::external_pkc::audit_outcome(conn, &pkc_outcome);
    if let Some(evidence) = pkc_outcome.evidence.as_ref() {
        retrieved.push(crate::external_pkc::package_model_safe_context(
            &evidence.model_safe_text,
        ));
    }
    let prompt = PromptAssembly::new(&identity, turn.routing.as_str(), &[], &retrieved);
    let system = prompt.render_system();
    let prompt_summary = prompt.inspection_view();

    let req = ChatRequest {
        system,
        messages: provider_messages,
        model: turn.model.clone(),
        max_tokens: turn.max_tokens,
        routing: turn.routing,
    };

    let provider_index = match decision {
        RoutingDecision::Use { provider_index } => provider_index,
        RoutingDecision::NeedsCloudConsent { provider_index } => {
            let provider = roster
                .get(provider_index)
                .ok_or(AppError::NotFound("selected provider missing".into()))?;
            let info = provider.info();
            let consent_req = CloudConsentRequest {
                provider_id: info.id.clone(),
                provider_display_name: info.display_name.clone(),
                model: turn.model.clone(),
                routing_mode: turn.routing.as_str().to_string(),
                estimated_prompt_tokens: provider.estimate_tokens(&req.system),
            };
            if !consent(&consent_req) {
                // Denied. Record an audit event and stop — no transmission.
                audit::record(
                    conn,
                    AuditCategory::Provider,
                    "cloud_crossing_denied",
                    None,
                    &serde_json::json!({
                        "provider": info.id,
                        "routing_mode": turn.routing.as_str(),
                    }),
                )?;
                return Err(AppError::Config(
                    "cloud crossing denied by owner; no transmission occurred".into(),
                ));
            }
            // Approved: audit the consent and proceed.
            audit::record(
                conn,
                AuditCategory::Provider,
                "cloud_crossing_approved",
                None,
                &serde_json::json!({
                    "provider": info.id,
                    "model": turn.model,
                }),
            )?;
            provider_index
        }
        RoutingDecision::NoProvider => {
            return Err(AppError::Config(format!(
                "no provider satisfies routing mode '{}'",
                turn.routing.as_str()
            )));
        }
    };

    let provider = roster
        .get(provider_index)
        .ok_or(AppError::NotFound("selected provider missing".into()))?;
    let info = provider.info();

    // 4. Call the provider.
    let response = provider.chat(&req)?;

    // 5. Record the assistant message.
    let assistant_id = conversation::append_message(
        conn,
        turn.conversation_id,
        Role::Assistant,
        &response.content,
    )?;
    let _ = crate::external_pkc::store_provenance(
        conn,
        &assistant_id.to_string(),
        &pkc_outcome.owner_view,
    );

    // 6. Audit: every cloud transmission is recorded; local calls are audited
    //    more lightly.
    let (action, detail) = if response.crossed_to_cloud {
        (
            "provider_cloud_call",
            serde_json::json!({
                "provider": info.id,
                "model": response.model,
                "routing_mode": turn.routing.as_str(),
            }),
        )
    } else {
        (
            "provider_local_call",
            serde_json::json!({
                "provider": info.id,
                "model": response.model,
            }),
        )
    };
    audit::record(conn, AuditCategory::Provider, action, None, &detail)?;

    let consent_requested = match decision {
        RoutingDecision::NeedsCloudConsent { .. } => Some(CloudConsentRequest {
            provider_id: info.id.clone(),
            provider_display_name: info.display_name.clone(),
            model: turn.model.clone(),
            routing_mode: turn.routing.as_str().to_string(),
            estimated_prompt_tokens: 0,
        }),
        _ => None,
    };

    Ok(ChatTurnResult {
        response,
        consent_requested,
        prompt_summary,
        pkc: pkc_outcome.owner_view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::MockProvider;

    fn fresh_conn() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE conversations (
                conversation_id TEXT PRIMARY KEY, title TEXT,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE messages (
                message_id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL,
                role TEXT NOT NULL, content TEXT NOT NULL,
                created_at TEXT NOT NULL, seq INTEGER NOT NULL);
             CREATE TABLE audit_events (
                seq INTEGER PRIMARY KEY AUTOINCREMENT, event_id TEXT NOT NULL UNIQUE,
                occurred_at TEXT NOT NULL, actor TEXT, category TEXT NOT NULL,
                action TEXT NOT NULL, detail_json TEXT NOT NULL DEFAULT '{}');
             CREATE TABLE identity (
                singleton INTEGER PRIMARY KEY, identity_json TEXT NOT NULL,
                updated_at TEXT NOT NULL);
             CREATE TABLE knowledge_records_full (
                record_id TEXT PRIMARY KEY, vault_id TEXT NOT NULL,
                record_type TEXT NOT NULL, canonical_text TEXT NOT NULL,
                status TEXT NOT NULL, source_ids TEXT NOT NULL DEFAULT '[]',
                source_locations TEXT NOT NULL DEFAULT '[]', provenance TEXT NOT NULL DEFAULT '{}',
                confidence REAL NOT NULL DEFAULT 0.0, sensitivity TEXT NOT NULL DEFAULT 'normal',
                permissions TEXT NOT NULL DEFAULT '[]', domain_tags TEXT NOT NULL DEFAULT '[]',
                routing_tags TEXT NOT NULL DEFAULT '[]', created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL, valid_from TEXT, valid_to TEXT,
                supersedes TEXT, superseded_by TEXT, review_state TEXT NOT NULL DEFAULT 'candidate',
                reviewed_by TEXT, reviewed_at TEXT, contradiction_set TEXT,
                schema_version INTEGER NOT NULL DEFAULT 1);
             CREATE VIRTUAL TABLE knowledge_fts USING fts5(
                record_id UNINDEXED, canonical_text);",
        )
        .unwrap();
        c
    }

    fn always_allow() -> ConsentFn {
        Box::new(|_| true)
    }
    fn always_deny() -> ConsentFn {
        Box::new(|_| false)
    }

    #[test]
    fn mock_turn_records_user_and_assistant_messages() {
        let conn = fresh_conn();
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hello sammy".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 64,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        // Mock echo includes the user text.
        assert!(result.response.content.contains("hello sammy"));
        assert!(!result.response.crossed_to_cloud);

        let msgs = conversation::messages(&conn, conv).unwrap();
        assert_eq!(msgs.len(), 2); // user + assistant
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn local_only_audits_local_call() {
        let conn = fresh_conn();
        let conv = conversation::create(&conn, None).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hi".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(audit::assert_action_recorded(&conn, "provider_local_call"));
        assert!(!audit::assert_action_recorded(&conn, "provider_cloud_call"));
    }

    #[test]
    fn cloud_crossing_with_denial_records_no_transmission() {
        let conn = fresh_conn();
        let conv = conversation::create(&conn, None).unwrap();
        // Roster: no local reachable, one cloud reachable.
        let roster = ProviderRoster {
            local: vec![],
            cloud: vec![Arc::new(MockProvider::fixed("should-not-happen"))],
        };
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "secret".into(),
            routing: RoutingMode::AskBeforeCrossing,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let err =
            run_turn(&conn, &roster, &[], &[true], &turn, &always_deny()).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
        assert!(audit::assert_action_recorded(
            &conn,
            "cloud_crossing_denied"
        ));
        assert!(!audit::assert_action_recorded(&conn, "provider_cloud_call"));
        // No assistant message recorded.
        let msgs = conversation::messages(&conn, conv).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
    }

    #[test]
    fn no_provider_returns_error() {
        let conn = fresh_conn();
        let conv = conversation::create(&conn, None).unwrap();
        let roster = ProviderRoster {
            local: vec![],
            cloud: vec![],
        };
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hi".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        assert!(run_turn(&conn, &roster, &[], &[], &turn, &always_allow()).is_err());
    }

    #[test]
    fn prompt_summary_returned_for_inspection() {
        let conn = fresh_conn();
        let conv = conversation::create(&conn, None).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hi".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!result.prompt_summary.is_empty());
        // Sanity: no secret leakage in the summary.
        let s = serde_json::to_string(&result.prompt_summary).unwrap();
        assert!(!s.contains("DEK"));
    }

    #[test]
    fn identity_persists_across_turns() {
        let conn = fresh_conn();
        let mut id = crate::identity::CompanionIdentity::default_new();
        id.companion_name = "TestSammy".into();
        identity::store::save(&conn, &id).unwrap();
        let conv = conversation::create(&conn, None).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hi".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        // The identity section title should include the saved name.
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        let identity_section = result
            .prompt_summary
            .iter()
            .find(|s| s.id == "identity")
            .unwrap();
        assert!(identity_section.title.contains("TestSammy"));
    }

    #[test]
    fn from_config_without_koboldcpp_is_mock_only() {
        let config = crate::settings::ProviderConfig::default();
        let roster = ProviderRoster::from_config(&config, None);
        assert_eq!(roster.local.len(), 1);
        assert_eq!(roster.local[0].info().id, "mock");
    }

    #[test]
    fn from_config_with_disabled_koboldcpp_is_mock_only() {
        let config = crate::settings::ProviderConfig {
            koboldcpp_endpoint: "http://localhost:5001".into(),
            koboldcpp_model: "test".into(),
            koboldcpp_enabled: false,
            ..Default::default()
        };
        let roster = ProviderRoster::from_config(&config, None);
        assert_eq!(roster.local.len(), 1);
        assert_eq!(roster.local[0].info().id, "mock");
    }

    #[test]
    fn from_config_with_enabled_koboldcpp_puts_it_first() {
        let config = crate::settings::ProviderConfig {
            koboldcpp_endpoint: "http://localhost:5001".into(),
            koboldcpp_model: "koboldcpp/test-model".into(),
            koboldcpp_enabled: true,
            ..Default::default()
        };
        let roster = ProviderRoster::from_config(&config, None);
        assert_eq!(roster.local.len(), 2);
        assert_eq!(roster.local[0].info().id, "koboldcpp");
        assert_eq!(roster.local[1].info().id, "mock");
    }

    #[test]
    fn from_config_adds_venice_only_when_enabled_with_key() {
        let config = crate::settings::ProviderConfig {
            venice_enabled: true,
            venice_model: "venice-model".into(),
            ..Default::default()
        };
        assert!(ProviderRoster::from_config(&config, None).cloud.is_empty());
        let roster = ProviderRoster::from_config(&config, Some("secret".into()));
        assert_eq!(roster.cloud.len(), 1);
        assert_eq!(roster.cloud[0].info().id, "venice");
    }

    #[test]
    fn from_config_with_empty_endpoint_is_mock_only() {
        // Enabled but no endpoint → don't add KoboldCpp.
        let config = crate::settings::ProviderConfig {
            koboldcpp_endpoint: "".into(),
            koboldcpp_model: "test".into(),
            koboldcpp_enabled: true,
            ..Default::default()
        };
        let roster = ProviderRoster::from_config(&config, None);
        assert_eq!(roster.local.len(), 1);
        assert_eq!(roster.local[0].info().id, "mock");
    }

    #[test]
    fn resolve_model_uses_configured_default_when_empty() {
        let config = crate::settings::ProviderConfig {
            koboldcpp_model: "configured-model".into(),
            ..Default::default()
        };
        assert_eq!(
            ProviderRoster::resolve_model(&config, ""),
            "configured-model"
        );
        // Non-empty requested model takes precedence.
        assert_eq!(
            ProviderRoster::resolve_model(&config, "user-model"),
            "user-model"
        );
    }

    #[test]
    fn run_turn_with_configured_roster_falls_back_to_mock_when_koboldcpp_unreachable() {
        // KoboldCpp at index 0 (unreachable), mock at index 1 (reachable).
        // With prefer_local or ask_before_crossing, the resolver should skip
        // the unreachable KoboldCpp and use the reachable mock.
        let conn = fresh_conn();
        let conv = conversation::create(&conn, None).unwrap();
        let config = crate::settings::ProviderConfig {
            koboldcpp_endpoint: "http://127.0.0.1:1".into(), // nothing listening
            koboldcpp_model: "test".into(),
            koboldcpp_enabled: true,
            ..Default::default()
        };
        let roster = ProviderRoster::from_config(&config, None);
        // local_reachable: [false (koboldcpp unreachable), true (mock)]
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hello".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "test".into(),
        };
        let result =
            run_turn(&conn, &roster, &[false, true], &[], &turn, &always_allow())
                .unwrap();
        // Should have fallen back to the mock.
        assert_eq!(result.response.provider, "mock");
    }

    fn fake_pkc_bridge(answer: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "sammy-pkc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake_bridge.py");
        let escaped = answer.replace('\\', "\\\\").replace('"', "\\\"");
        std::fs::write(
            &script,
            format!(
                r#"import json, pathlib, sys
pathlib.Path(sys.argv[0]).with_suffix(".ran").write_text("ran", encoding="utf-8")
print(json.dumps({{
  "bridge_version": "1.0.0",
  "ok": True,
  "available": True,
  "payload": {{
    "natural_answer": "{escaped}",
    "conversational_context": "",
    "authorized": True,
    "canonical_hash_verified": True,
    "protected_content_materialized": True
  }}
}}))
"#
            ),
        )
        .unwrap();
        (script, dir.join("fake_bridge.ran"))
    }

    fn enable_pkc(conn: &rusqlite::Connection, script: &std::path::Path) {
        let cfg = crate::settings::ProviderConfig {
            pkc_enabled: true,
            pkc_python_executable: "python".into(),
            pkc_bridge_script: script.to_string_lossy().into_owned(),
            pkc_source_id: "SRC-SHA256-test".into(),
            ..Default::default()
        };
        crate::settings::save(conn, &cfg).unwrap();
    }

    fn knowledge_count(conn: &rusqlite::Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn knowledge_chars(result: &ChatTurnResult) -> u32 {
        result
            .prompt_summary
            .iter()
            .find(|s| s.id == "knowledge")
            .unwrap()
            .body_chars
    }

    #[test]
    fn enabled_pkc_retrieval_does_not_write_durable_memory() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("Dave grew up in Gloucester Township.");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 64,
            model: "mock-1".into(),
        };
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(ran.exists(), "local turn should query the PKC bridge");
        assert!(knowledge_chars(&result) > empty);
        assert_eq!(knowledge_count(&conn), 0);
        assert!(!result.response.crossed_to_cloud);
        assert_eq!(result.response.provider, "mock");
        assert!(result.pkc.used);
        assert_eq!(result.pkc.classification.as_deref(), Some("stored_fact"));
        let audit_blob: String = conn
            .prepare("SELECT group_concat(detail_json, '\n') FROM audit_events")
            .unwrap()
            .query_row([], |r| r.get::<_, Option<String>>(0))
            .unwrap()
            .unwrap_or_default();
        assert!(!audit_blob.contains("Gloucester"));
        assert!(!audit_blob.contains("Dave grew up"));
        crate::memory::ensure_schema(&conn).unwrap();
        let mem: i64 = conn
            .query_row("SELECT count(*) FROM memory_candidates", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mem, 0);
    }

    #[test]
    fn repeated_pkc_retrieval_still_writes_no_durable_memory() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("Dave grew up in Gloucester Township.");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        for _ in 0..2 {
            let turn = ChatTurn {
                conversation_id: conv,
                user_text: "Where did I grow up?".into(),
                routing: RoutingMode::LocalOnly,
                max_tokens: 32,
                model: "mock-1".into(),
            };
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        }
        assert!(ran.exists());
        assert_eq!(knowledge_count(&conn), 0);
        crate::memory::ensure_schema(&conn).unwrap();
        let mem: i64 = conn
            .query_row("SELECT count(*) FROM memory_candidates", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mem, 0);
    }

    #[test]
    fn owner_approved_memory_follows_normal_review_path_with_provenance() {
        let conn = fresh_conn();
        crate::memory::ensure_schema(&conn).unwrap();
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let msgs = conversation::messages(&conn, conv).unwrap();
        let candidate = crate::memory::propose(
            &conn,
            "Grew up in Gloucester Township",
            "preference",
            Some(conv),
            None,
            "owner review of a PKC-grounded chat turn",
            0.8,
            "normal",
            "personal",
            "mock",
            false,
        )
        .unwrap();
        assert_eq!(knowledge_count(&conn), 0);
        let _ = msgs;
        let _ = candidate;
        let mem: i64 = conn
            .query_row("SELECT count(*) FROM memory_candidates", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mem, 1);
        assert_eq!(knowledge_count(&conn), 0);
    }

    #[test]
    fn greeting_does_not_query_pkc() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("MUST_NOT_APPEAR");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "hello".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "greetings must not query PKC");
        assert!(!result.pkc.used);
        assert_eq!(result.pkc.skip_reason.as_deref(), Some("not_useful"));
        let _ = script;
    }

    #[test]
    fn prefer_cloud_does_not_query_pkc_even_when_local_is_up() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("MUST_NOT_CROSS_TO_CLOUD");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster {
            local: vec![Arc::new(MockProvider::echo())],
            cloud: vec![Arc::new(MockProvider::fixed("cloud-answer"))],
        };
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::PreferCloud,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[true], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "cloud route must not query PKC");
        assert!(!result.pkc.used);
        assert_eq!(result.pkc.state, "cloud_turn_skipped");
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        assert_eq!(knowledge_chars(&result), empty);
        assert!(!result.response.content.contains("MUST_NOT_CROSS_TO_CLOUD"));
    }

    #[test]
    fn local_to_cloud_fallback_does_not_carry_pkc_evidence() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("MUST_NOT_CROSS_TO_CLOUD");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        // No local provider; PreferLocal therefore requires cloud consent.
        let roster = ProviderRoster {
            local: vec![],
            cloud: vec![Arc::new(MockProvider::fixed("cloud-answer"))],
        };
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::PreferLocal,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[], &[true], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "fallback cloud turn must not invoke PKC");
        assert!(!result.pkc.used);
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        assert_eq!(knowledge_chars(&result), empty);
        assert!(!result.response.content.contains("MUST_NOT_CROSS_TO_CLOUD"));
    }

    #[test]
    fn sparse_pkc_evidence_is_packaged_without_scene_license() {
        let conn = fresh_conn();
        let (script, _) = fake_pkc_bridge("The family moved often.");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "What do you know about my childhood?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 32,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(result.pkc.used);
        assert!(knowledge_chars(&result) > 40);
        assert!(crate::grounding::presents_unsupported_concrete_scene(
            "The way the light shifted as you moved through that narrow doorway sounds like a very specific moment.",
            "The family moved often.",
            "What do you know about my childhood?",
        ));
        assert!(audit::assert_action_recorded(
            &conn,
            "pkc_retrieval_succeeded"
        ));
        assert!(audit::assert_action_recorded(
            &conn,
            "pkc_evidence_sanitized"
        ));
    }

    #[test]
    fn malformed_pkc_response_degrades_without_crashing() {
        let conn = fresh_conn();
        let dir = std::env::temp_dir().join(format!(
            "sammy-pkc-bad-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake_bridge.py");
        std::fs::write(&script, "print('not-json')\n").unwrap();
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!result.pkc.used);
        assert!(result.pkc.owner_notice.is_some());
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        assert_eq!(knowledge_chars(&result), empty);
    }

    #[test]
    fn unauthorized_pkc_evidence_never_reaches_the_model() {
        let conn = fresh_conn();
        let dir = std::env::temp_dir().join(format!(
            "sammy-pkc-unauth-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake_bridge.py");
        std::fs::write(
            &script,
            r#"import json
print(json.dumps({
  "bridge_version": "1.0.0",
  "ok": True,
  "available": True,
  "payload": {
    "natural_answer": "MUST_NOT_REACH_MODEL",
    "conversational_context": "",
    "authorized": False,
    "canonical_hash_verified": False,
    "protected_content_materialized": False
  }
}))
"#,
        )
        .unwrap();
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!result.pkc.used);
        assert_eq!(result.pkc.state, "available_unauthorized");
        assert!(!result.response.content.contains("MUST_NOT_REACH_MODEL"));
        assert!(audit::assert_action_recorded(
            &conn,
            "pkc_authorization_denied"
        ));
    }

    #[test]
    fn disable_mid_session_stops_retrieval() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("Dave grew up in Gloucester Township.");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(ran.exists());
        std::fs::remove_file(&ran).ok();
        crate::settings::save(&conn, &crate::settings::ProviderConfig::default())
            .unwrap();
        run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "disabled gate must stop further retrieval");
    }

    #[test]
    fn disabled_pkc_gate_restores_normal_behavior() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("MUST_NOT_APPEAR");
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster::mock_only();
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::LocalOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[true], &[], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "disabled gate must not spawn the bridge");
        let _ = script;
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        assert_eq!(knowledge_chars(&result), empty);
        assert_eq!(knowledge_count(&conn), 0);
    }

    #[test]
    fn cloud_bound_turns_do_not_receive_pkc_evidence() {
        let conn = fresh_conn();
        let (script, ran) = fake_pkc_bridge("MUST_NOT_CROSS_TO_CLOUD");
        enable_pkc(&conn, &script);
        let conv = conversation::create(&conn, Some("c")).unwrap();
        let roster = ProviderRoster {
            local: vec![],
            cloud: vec![Arc::new(MockProvider::fixed("cloud-answer"))],
        };
        let turn = ChatTurn {
            conversation_id: conv,
            user_text: "Where did I grow up?".into(),
            routing: RoutingMode::CloudOnly,
            max_tokens: 16,
            model: "mock-1".into(),
        };
        let result =
            run_turn(&conn, &roster, &[], &[true], &turn, &always_allow()).unwrap();
        assert!(!ran.exists(), "cloud-bound turns must not query PKC");
        let empty = "(no approved knowledge retrieved for this turn)".len() as u32;
        assert_eq!(knowledge_chars(&result), empty);
        assert_eq!(knowledge_count(&conn), 0);
    }
}
