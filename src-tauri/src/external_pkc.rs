//! Read-only adapter to the external Personal Knowledge Corpus (PKC).
//!
//! This is **not** Sammy's internal vault corpus (`corpus` / `retrieval`).
//! PKC is a separate durable-knowledge product. Sammy queries it through the
//! same authorized Python gateway the PKC Reference Client uses, then sanitizes
//! the conversational payload before any model sees it.
//!
//! Constraints:
//! - feature-gated (`pkc_enabled`, default off)
//! - read-only: no durable memory writes, no corpus mutation
//! - unauthorized / unavailable evidence never reaches the model
//! - PKC bookkeeping must not enter user-facing or model-facing text
//! - no cloud path: callers skip this adapter until a **local** route is committed
//! - not every local message queries PKC; see [`should_retrieve_for_turn`]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::grounding;
use crate::settings::ProviderConfig;

const BRIDGE_VERSION: &str = "1.0.0";
/// PKC consumer identity. Displayed in Settings; sent on every request.
pub const CONSUMER_APPLICATION: &str = "sammy";
/// Authorized purpose for Sammy. Displayed in Settings; sent on every request.
pub const PURPOSE: &str = "personal_consigliere";

const HEALTH_TIMEOUT_SECS: u64 = 12;
const RETRIEVAL_TIMEOUT_SECS: u64 = 20;

/// Optional developer override. The value is never a baked-in machine path.
const ENV_PKC_BRIDGE: &str = "SAMMY_PKC_BRIDGE";
const ENV_PKC_ROOT: &str = "SAMMY_PKC_ROOT";

/// Conversational payload allowed into the prompt, or `None` to degrade safely.
pub fn authorized_payload_for_turn(
    cfg: &ProviderConfig,
    question: &str,
) -> Option<String> {
    consult_without_judgment(cfg, question)
        .evidence
        .map(|e| e.model_safe_text)
}

/// Parse a PKC Reference Client-shaped bridge response. Returns conversational
/// text only when authorized, materialized, and free of bookkeeping.
pub fn conversational_payload_from_bridge_json(raw: &str) -> Option<String> {
    sanitize_bridge_response(raw).map(|evidence| evidence.model_safe_text)
}

/// Lightweight decision: is PKC retrieval useful for this owner message?
///
/// Retrieve for questions about the owner, past events, preferences, project
/// history, relationships among stored information, or explicit recall.
/// Skip greetings, thanks, pure arithmetic, and generic world facts.
/// Conservative: no personal cue → no lookup.
pub fn should_retrieve_for_turn(question: &str) -> bool {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if is_greeting_or_thanks(&lower) {
        return false;
    }
    if is_pure_arithmetic(&lower) {
        return false;
    }
    has_personal_knowledge_cue(&lower)
}

/// First-class health / turn states. A green connected indicator requires
/// [`PkcHealthState::AvailableAuthorized`] after an actual probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PkcHealthState {
    Disabled,
    ConfiguredNotTested,
    Connecting,
    AvailableAuthorized,
    AvailableUnauthorized,
    PkcUnavailable,
    BridgeUnavailable,
    PythonUnavailable,
    Misconfigured,
    LocalModelUnavailable,
    CloudTurnSkipped,
    RetrievalSkipped,
}

impl PkcHealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::ConfiguredNotTested => "configured_not_tested",
            Self::Connecting => "connecting",
            Self::AvailableAuthorized => "available_authorized",
            Self::AvailableUnauthorized => "available_unauthorized",
            Self::PkcUnavailable => "pkc_unavailable",
            Self::BridgeUnavailable => "bridge_unavailable",
            Self::PythonUnavailable => "python_unavailable",
            Self::Misconfigured => "misconfigured",
            Self::LocalModelUnavailable => "local_model_unavailable",
            Self::CloudTurnSkipped => "cloud_turn_skipped",
            Self::RetrievalSkipped => "retrieval_skipped",
        }
    }

    pub fn owner_message(self) -> &'static str {
        match self {
            Self::Disabled => "Personal knowledge is off. Enable it in Settings to use it in local chat.",
            Self::ConfiguredNotTested => {
                "Settings are saved but not yet verified. Use Test connection."
            }
            Self::Connecting => "Checking the personal-knowledge connection…",
            Self::AvailableAuthorized => {
                "Personal knowledge is available and authorized for Sammy."
            }
            Self::AvailableUnauthorized => {
                "The knowledge system answered, but Sammy is not authorized for this source."
            }
            Self::PkcUnavailable => {
                "The personal-knowledge system is not reachable. Sammy chat still works without it."
            }
            Self::BridgeUnavailable => {
                "The PKC bridge script is missing or moved. Chat still works without personal knowledge."
            }
            Self::PythonUnavailable => {
                "Python was not found. Chat still works without personal knowledge."
            }
            Self::Misconfigured => {
                "Personal-knowledge settings are incomplete or invalid. Chat still works."
            }
            Self::LocalModelUnavailable => {
                "The local model is not reachable. Personal knowledge is not sent to cloud models."
            }
            Self::CloudTurnSkipped => {
                "This turn uses a cloud model, so personal knowledge was not consulted."
            }
            Self::RetrievalSkipped => {
                "This message did not need a personal-knowledge lookup."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    StoredFact,
    Testimony,
    Inference,
}

impl EvidenceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StoredFact => "stored_fact",
            Self::Testimony => "testimony",
            Self::Inference => "inference",
        }
    }
}

/// Structured evidence after the sanitization boundary. The model sees only
/// [`PkcEvidence::model_safe_text`]. The owner view never includes the body.
#[derive(Debug, Clone)]
pub struct PkcEvidence {
    pub source_id: String,
    pub source_type: &'static str,
    pub classification: EvidenceClass,
    pub authorized: bool,
    pub request_id: String,
    pub payload_sha256: String,
    pub payload_chars: u32,
    pub model_safe_text: String,
}

/// Body-free view for Chat UI, audit, and persistence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PkcTurnView {
    pub used: bool,
    pub state: String,
    pub owner_notice: Option<String>,
    pub source_id: Option<String>,
    pub classification: Option<String>,
    pub request_id: Option<String>,
    pub payload_sha256: Option<String>,
    pub payload_chars: u32,
    pub authorized: Option<bool>,
    pub skip_reason: Option<String>,
}

impl Default for PkcTurnView {
    fn default() -> Self {
        Self {
            used: false,
            state: PkcHealthState::Disabled.as_str().into(),
            owner_notice: None,
            source_id: None,
            classification: None,
            request_id: None,
            payload_sha256: None,
            payload_chars: 0,
            authorized: None,
            skip_reason: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PkcTurnOutcome {
    pub state: PkcHealthState,
    pub used: bool,
    pub skip_reason: Option<&'static str>,
    pub evidence: Option<PkcEvidence>,
    pub owner_view: PkcTurnView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkcDiscovery {
    pub python_executable: String,
    pub bridge_script: String,
    pub pkc_root: String,
    pub source_id: String,
    pub consumer: String,
    pub purpose: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkcHealthReport {
    pub state: PkcHealthState,
    pub owner_message: String,
    pub consumer: String,
    pub purpose: String,
    pub local_only: bool,
    pub python_ok: bool,
    pub bridge_ok: bool,
    pub root_ok: bool,
    pub source_configured: bool,
    pub authorized: Option<bool>,
    pub probed: bool,
    pub payload_chars: u32,
    pub payload_sha256: Option<String>,
}

/// Discover reasonable local defaults without requiring JSON editing.
pub fn discover_defaults() -> PkcDiscovery {
    let mut notes = Vec::new();
    let python = discover_python();
    if python.is_empty() {
        notes.push(
            "Python was not found on PATH. Install Python or set the executable.".into(),
        );
    }
    let bridge = existing_path_from_env(ENV_PKC_BRIDGE, true);
    if bridge.is_empty() {
        notes.push(
            "No PKC bridge script was found. Choose the bridge file in Settings.".into(),
        );
    }
    let root = existing_path_from_env(ENV_PKC_ROOT, false);
    if root.is_empty() {
        notes.push(
            "No PKC root folder was found. Choose the PKC folder in Settings.".into(),
        );
    }
    let source_id = if root.is_empty() {
        String::new()
    } else {
        discover_source_id(Path::new(&root)).unwrap_or_default()
    };
    if source_id.is_empty() {
        notes.push("No source identity was discovered. Choose the source Sammy is allowed to read.".into());
    }
    PkcDiscovery {
        python_executable: python,
        bridge_script: bridge,
        pkc_root: root,
        source_id,
        consumer: CONSUMER_APPLICATION.into(),
        purpose: PURPOSE.into(),
        notes,
    }
}

/// Validate settings the owner is about to save. Incomplete config is allowed
/// while the feature is off.
pub fn validate_settings(cfg: &ProviderConfig) -> Result<(), String> {
    if !cfg.pkc_enabled {
        return Ok(());
    }
    if cfg.pkc_source_id.trim().is_empty() {
        return Err(
            "Choose a PKC source identity before enabling personal knowledge.".into(),
        );
    }
    let script = cfg.pkc_bridge_script.trim();
    if script.is_empty() {
        return Err(
            "Set the PKC bridge location before enabling personal knowledge.".into(),
        );
    }
    let path = Path::new(script);
    if !path.is_absolute() {
        return Err("The PKC bridge path must be an absolute file path.".into());
    }
    if !path.is_file() {
        return Err("The PKC bridge file was not found at that location.".into());
    }
    let root = cfg.pkc_root.trim();
    if !root.is_empty() {
        let root_path = Path::new(root);
        if !root_path.is_absolute() {
            return Err("The PKC location must be an absolute folder path.".into());
        }
        if !root_path.is_dir() {
            return Err("The PKC location was not found. It may have been moved.".into());
        }
    }
    let python = python_exe(cfg);
    if Path::new(python).is_absolute() && !Path::new(python).exists() {
        return Err("The Python executable was not found at that location.".into());
    }
    Ok(())
}

/// Lightweight health check. When `probe` is true, actually invokes the
/// authorized gateway and discards corpus text. Never returns private bodies.
pub fn health_check(cfg: &ProviderConfig, probe: bool) -> PkcHealthReport {
    let python = python_exe(cfg);
    let python_ok = python_available(python);
    let bridge_ok = Path::new(cfg.pkc_bridge_script.trim()).is_file();
    let root_ok =
        cfg.pkc_root.trim().is_empty() || Path::new(cfg.pkc_root.trim()).is_dir();
    let source_configured = !cfg.pkc_source_id.trim().is_empty();

    if !probe {
        let state = if !cfg.pkc_enabled {
            PkcHealthState::Disabled
        } else if !cfg.pkc_last_health_state.is_empty() {
            parse_health_state(&cfg.pkc_last_health_state)
                .unwrap_or(PkcHealthState::ConfiguredNotTested)
        } else {
            PkcHealthState::ConfiguredNotTested
        };
        return PkcHealthReport {
            state,
            owner_message: state.owner_message().into(),
            consumer: CONSUMER_APPLICATION.into(),
            purpose: PURPOSE.into(),
            local_only: true,
            python_ok,
            bridge_ok,
            root_ok,
            source_configured,
            authorized: None,
            probed: false,
            payload_chars: 0,
            payload_sha256: None,
        };
    }

    let preflight =
        preflight_state(cfg, python_ok, bridge_ok, root_ok, source_configured);
    if let Some(state) = preflight {
        return PkcHealthReport {
            state,
            owner_message: state.owner_message().into(),
            consumer: CONSUMER_APPLICATION.into(),
            purpose: PURPOSE.into(),
            local_only: true,
            python_ok,
            bridge_ok,
            root_ok,
            source_configured,
            authorized: None,
            probed: false,
            payload_chars: 0,
            payload_sha256: None,
        };
    }

    let outcome = invoke_and_sanitize(cfg, "Authorization probe", HEALTH_TIMEOUT_SECS);
    let (state, authorized, chars, hash) = match outcome {
        Ok(evidence) => (
            PkcHealthState::AvailableAuthorized,
            Some(true),
            evidence.payload_chars,
            Some(evidence.payload_sha256),
        ),
        Err(BridgeFailure::Unauthorized) => {
            (PkcHealthState::AvailableUnauthorized, Some(false), 0, None)
        }
        Err(BridgeFailure::PythonUnavailable) => {
            (PkcHealthState::PythonUnavailable, None, 0, None)
        }
        Err(BridgeFailure::BridgeUnavailable) => {
            (PkcHealthState::BridgeUnavailable, None, 0, None)
        }
        Err(BridgeFailure::Unavailable) | Err(BridgeFailure::Timeout) => {
            (PkcHealthState::PkcUnavailable, None, 0, None)
        }
        Err(_) => (PkcHealthState::Misconfigured, None, 0, None),
    };
    let owner_message = if !cfg.pkc_enabled
        && state == PkcHealthState::AvailableAuthorized
    {
        "Personal knowledge answered and is authorized. Enable it to use in local chat."
            .into()
    } else {
        state.owner_message().into()
    };
    PkcHealthReport {
        state,
        owner_message,
        consumer: CONSUMER_APPLICATION.into(),
        purpose: PURPOSE.into(),
        local_only: true,
        python_ok,
        bridge_ok,
        root_ok,
        source_configured,
        authorized,
        probed: true,
        payload_chars: chars,
        payload_sha256: hash,
    }
}

/// Skip PKC because the committed route is cloud (or not local).
pub fn skipped_cloud_bound() -> PkcTurnOutcome {
    outcome(
        PkcHealthState::CloudTurnSkipped,
        false,
        Some("cloud_bound"),
        None,
        false,
    )
}

/// Consult PKC only after a local provider route is committed.
pub fn consult_for_local_turn(cfg: &ProviderConfig, question: &str) -> PkcTurnOutcome {
    if !cfg.pkc_enabled {
        return outcome(
            PkcHealthState::Disabled,
            false,
            Some("disabled"),
            None,
            false,
        );
    }
    if !should_retrieve_for_turn(question) {
        return outcome(
            PkcHealthState::RetrievalSkipped,
            false,
            Some("not_useful"),
            None,
            false,
        );
    }
    let python = python_exe(cfg);
    let python_ok = python_available(python);
    let bridge_ok = Path::new(cfg.pkc_bridge_script.trim()).is_file();
    let root_ok =
        cfg.pkc_root.trim().is_empty() || Path::new(cfg.pkc_root.trim()).is_dir();
    let source_configured = !cfg.pkc_source_id.trim().is_empty();
    if let Some(state) =
        preflight_state(cfg, python_ok, bridge_ok, root_ok, source_configured)
    {
        return outcome(state, false, Some(state.as_str()), None, true);
    }
    consult_without_judgment(cfg, question)
}

fn consult_without_judgment(cfg: &ProviderConfig, question: &str) -> PkcTurnOutcome {
    if !cfg.pkc_enabled {
        return outcome(
            PkcHealthState::Disabled,
            false,
            Some("disabled"),
            None,
            false,
        );
    }
    match invoke_and_sanitize(cfg, question, RETRIEVAL_TIMEOUT_SECS) {
        Ok(evidence) => {
            let view = view_from_evidence(&evidence, PkcHealthState::AvailableAuthorized);
            PkcTurnOutcome {
                state: PkcHealthState::AvailableAuthorized,
                used: true,
                skip_reason: None,
                owner_view: view,
                evidence: Some(evidence),
            }
        }
        Err(BridgeFailure::Unauthorized) => outcome(
            PkcHealthState::AvailableUnauthorized,
            false,
            Some("unauthorized"),
            None,
            true,
        ),
        Err(BridgeFailure::PythonUnavailable) => outcome(
            PkcHealthState::PythonUnavailable,
            false,
            Some("python_unavailable"),
            None,
            true,
        ),
        Err(BridgeFailure::BridgeUnavailable) => outcome(
            PkcHealthState::BridgeUnavailable,
            false,
            Some("bridge_unavailable"),
            None,
            true,
        ),
        Err(BridgeFailure::Timeout) | Err(BridgeFailure::Unavailable) => outcome(
            PkcHealthState::PkcUnavailable,
            false,
            Some("pkc_unavailable"),
            None,
            true,
        ),
        Err(BridgeFailure::Empty)
        | Err(BridgeFailure::Bookkeeping)
        | Err(BridgeFailure::Malformed) => outcome(
            PkcHealthState::Misconfigured,
            false,
            Some("misconfigured"),
            None,
            true,
        ),
    }
}

/// Model-visible packaging: epistemic classes without bookkeeping.
pub fn package_model_safe_context(sanitized_payload: &str) -> String {
    format!(
        "[personal knowledge — stored/source-backed fact]\n\
Use the block below as stored knowledge from an authorized personal corpus.\n\
Present stored facts as stored knowledge when they are supported.\n\
If referring to the owner's own words from this conversation, attribute them as testimony \
(\"You previously described…\") rather than as independently verified fact.\n\
If you reason beyond the stored evidence, frame it as inference (\"That suggests…\") — \
not as source truth.\n\
Do not invent concrete scene details (doorway, lighting, weather, facial expression, smell, \
exact sequence) as if the corpus supplied them.\n\
---\n{sanitized_payload}"
    )
}

pub fn ensure_provenance_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_pkc_provenance (
            message_id TEXT PRIMARY KEY,
            used INTEGER NOT NULL,
            state TEXT NOT NULL,
            source_id TEXT,
            classification TEXT,
            request_id TEXT,
            payload_sha256 TEXT,
            payload_chars INTEGER NOT NULL DEFAULT 0,
            authorized INTEGER,
            skip_reason TEXT,
            owner_notice TEXT,
            created_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

pub fn store_provenance(
    conn: &Connection,
    message_id: &str,
    view: &PkcTurnView,
) -> AppResult<()> {
    ensure_provenance_schema(conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO message_pkc_provenance(
            message_id, used, state, source_id, classification, request_id,
            payload_sha256, payload_chars, authorized, skip_reason, owner_notice, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(message_id) DO UPDATE SET
            used=excluded.used, state=excluded.state, source_id=excluded.source_id,
            classification=excluded.classification, request_id=excluded.request_id,
            payload_sha256=excluded.payload_sha256, payload_chars=excluded.payload_chars,
            authorized=excluded.authorized, skip_reason=excluded.skip_reason,
            owner_notice=excluded.owner_notice, created_at=excluded.created_at",
        params![
            message_id,
            view.used as i64,
            view.state,
            view.source_id,
            view.classification,
            view.request_id,
            view.payload_sha256,
            view.payload_chars as i64,
            view.authorized.map(|v| v as i64),
            view.skip_reason,
            view.owner_notice,
            now,
        ],
    )?;
    Ok(())
}

pub fn load_provenance(conn: &Connection, message_id: &str) -> Option<PkcTurnView> {
    ensure_provenance_schema(conn).ok()?;
    conn.query_row(
        "SELECT used, state, source_id, classification, request_id, payload_sha256,
                payload_chars, authorized, skip_reason, owner_notice
         FROM message_pkc_provenance WHERE message_id = ?1",
        params![message_id],
        |r| {
            Ok(PkcTurnView {
                used: r.get::<_, i64>(0)? != 0,
                state: r.get(1)?,
                source_id: r.get(2)?,
                classification: r.get(3)?,
                request_id: r.get(4)?,
                payload_sha256: r.get(5)?,
                payload_chars: r.get::<_, i64>(6)? as u32,
                authorized: r.get::<_, Option<i64>>(7)?.map(|v| v != 0),
                skip_reason: r.get(8)?,
                owner_notice: r.get(9)?,
            })
        },
    )
    .ok()
}

pub fn audit_outcome(conn: &Connection, outcome: &PkcTurnOutcome) -> AppResult<()> {
    let action = if outcome.used {
        "pkc_retrieval_succeeded"
    } else if outcome.skip_reason == Some("cloud_bound") {
        "pkc_retrieval_skipped_cloud"
    } else if outcome.skip_reason == Some("not_useful") {
        "pkc_retrieval_skipped_judgment"
    } else if outcome.skip_reason == Some("disabled") {
        "pkc_retrieval_skipped_disabled"
    } else if matches!(outcome.state, PkcHealthState::AvailableUnauthorized) {
        "pkc_authorization_denied"
    } else {
        "pkc_retrieval_unavailable"
    };
    let detail = json!({
        "state": outcome.state.as_str(),
        "used": outcome.used,
        "skip_reason": outcome.skip_reason,
        "authorized": outcome.owner_view.authorized,
        "payload_chars": outcome.owner_view.payload_chars,
        "payload_sha256": outcome.owner_view.payload_sha256,
        "request_id": outcome.owner_view.request_id,
        "source_id": outcome.owner_view.source_id,
        "consumer": CONSUMER_APPLICATION,
        "purpose": PURPOSE,
    });
    crate::audit::record(
        conn,
        crate::audit::AuditCategory::Knowledge,
        action,
        None,
        &detail,
    )?;
    if outcome.used {
        crate::audit::record(
            conn,
            crate::audit::AuditCategory::Knowledge,
            "pkc_evidence_sanitized",
            None,
            &json!({
                "payload_chars": outcome.owner_view.payload_chars,
                "payload_sha256": outcome.owner_view.payload_sha256,
                "request_id": outcome.owner_view.request_id,
            }),
        )?;
    }
    Ok(())
}

fn outcome(
    state: PkcHealthState,
    used: bool,
    skip_reason: Option<&'static str>,
    evidence: Option<PkcEvidence>,
    notice: bool,
) -> PkcTurnOutcome {
    let owner_notice = if notice {
        Some(state.owner_message().to_string())
    } else {
        None
    };
    let owner_view = PkcTurnView {
        used,
        state: state.as_str().into(),
        owner_notice,
        source_id: evidence.as_ref().map(|e| e.source_id.clone()),
        classification: evidence.as_ref().map(|e| e.classification.as_str().into()),
        request_id: evidence.as_ref().map(|e| e.request_id.clone()),
        payload_sha256: evidence.as_ref().map(|e| e.payload_sha256.clone()),
        payload_chars: evidence.as_ref().map(|e| e.payload_chars).unwrap_or(0),
        authorized: if used {
            Some(true)
        } else if skip_reason == Some("unauthorized") {
            Some(false)
        } else {
            None
        },
        skip_reason: skip_reason.map(|s| s.to_string()),
    };
    PkcTurnOutcome {
        state,
        used,
        skip_reason,
        evidence,
        owner_view,
    }
}

fn view_from_evidence(evidence: &PkcEvidence, state: PkcHealthState) -> PkcTurnView {
    PkcTurnView {
        used: true,
        state: state.as_str().into(),
        owner_notice: None,
        source_id: Some(evidence.source_id.clone()),
        classification: Some(evidence.classification.as_str().into()),
        request_id: Some(evidence.request_id.clone()),
        payload_sha256: Some(evidence.payload_sha256.clone()),
        payload_chars: evidence.payload_chars,
        authorized: Some(true),
        skip_reason: None,
    }
}

fn preflight_state(
    cfg: &ProviderConfig,
    python_ok: bool,
    bridge_ok: bool,
    root_ok: bool,
    source_configured: bool,
) -> Option<PkcHealthState> {
    if cfg.pkc_bridge_script.trim().is_empty() || !source_configured {
        return Some(PkcHealthState::Misconfigured);
    }
    if Path::new(python_exe(cfg)).is_absolute() && !python_ok {
        return Some(PkcHealthState::PythonUnavailable);
    }
    if !bridge_ok {
        return Some(PkcHealthState::BridgeUnavailable);
    }
    if !root_ok {
        return Some(PkcHealthState::PkcUnavailable);
    }
    None
}

#[derive(Debug)]
enum BridgeFailure {
    PythonUnavailable,
    BridgeUnavailable,
    Timeout,
    Malformed,
    Unavailable,
    Unauthorized,
    Empty,
    Bookkeeping,
}

fn invoke_and_sanitize(
    cfg: &ProviderConfig,
    question: &str,
    timeout_secs: u64,
) -> Result<PkcEvidence, BridgeFailure> {
    let request_id = format!("SAMMY-PKC-{}", uuid::Uuid::new_v4());
    let raw = invoke_bridge(cfg, question, &request_id, timeout_secs)?;
    let mut evidence = parse_sanitized(&raw)?;
    evidence.request_id = request_id;
    evidence.source_id = cfg.pkc_source_id.trim().to_string();
    Ok(evidence)
}

/// Sanitization boundary: raw PKC JSON → authorized, bookkeeping-free evidence.
pub fn sanitize_bridge_response(raw: &str) -> Option<PkcEvidence> {
    parse_sanitized(raw).ok()
}

fn parse_sanitized(raw: &str) -> Result<PkcEvidence, BridgeFailure> {
    let value: Value = serde_json::from_str(raw).map_err(|_| BridgeFailure::Malformed)?;
    if value.get("bridge_version").and_then(Value::as_str) != Some(BRIDGE_VERSION) {
        return Err(BridgeFailure::Malformed);
    }
    if value.get("ok").and_then(Value::as_bool) != Some(true)
        || value.get("available").and_then(Value::as_bool) != Some(true)
    {
        return Err(BridgeFailure::Unavailable);
    }
    let payload = value
        .get("payload")
        .cloned()
        .ok_or(BridgeFailure::Malformed)?;
    if payload.get("authorized").and_then(Value::as_bool) != Some(true) {
        return Err(BridgeFailure::Unauthorized);
    }
    if payload
        .get("canonical_hash_verified")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(BridgeFailure::Unauthorized);
    }
    if payload
        .get("protected_content_materialized")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(BridgeFailure::Unauthorized);
    }
    let natural = payload
        .get("natural_answer")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let context = payload
        .get("conversational_context")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let combined = [natural, context]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if combined.is_empty() {
        return Err(BridgeFailure::Empty);
    }
    if grounding::leaks_pkc_bookkeeping(&combined) {
        return Err(BridgeFailure::Bookkeeping);
    }
    let hash = hex::encode(Sha256::digest(combined.as_bytes()));
    Ok(PkcEvidence {
        source_id: String::new(),
        source_type: "external_pkc",
        classification: EvidenceClass::StoredFact,
        authorized: true,
        request_id: String::new(),
        payload_sha256: hash,
        payload_chars: combined.chars().count() as u32,
        model_safe_text: combined,
    })
}

fn invoke_bridge(
    cfg: &ProviderConfig,
    question: &str,
    request_id: &str,
    timeout_secs: u64,
) -> Result<String, BridgeFailure> {
    let script = Path::new(&cfg.pkc_bridge_script);
    if !script.is_absolute() || !script.is_file() {
        return Err(BridgeFailure::BridgeUnavailable);
    }
    let python = python_exe(cfg);
    if Path::new(python).is_absolute() && !Path::new(python).exists() {
        return Err(BridgeFailure::PythonUnavailable);
    }
    let request = json!({
        "bridge_version": BRIDGE_VERSION,
        "request_id": request_id,
        "operation": "protected_retrieval",
        "question": question,
        "chapter_context": "Personal question — Sammy read-only PKC consult",
        "consumer_application": CONSUMER_APPLICATION,
        "recipient_class": "owner_dave",
        "realm": "private_autobiographical_interview",
        "purpose": PURPOSE,
        "disclosure_mode": "private_local",
        "requested_content_level": "full_text",
        "consent_state": "standing_authorization",
        "source_id": cfg.pkc_source_id.trim(),
    });

    let mut cmd = Command::new(python);
    cmd.arg(script)
        .arg("--timeout-seconds")
        .arg(timeout_secs.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = script.parent() {
        cmd.current_dir(dir);
    }
    if !cfg.pkc_root.trim().is_empty() {
        cmd.arg("--pkc-root").arg(cfg.pkc_root.trim());
    }

    let mut child = cmd.spawn().map_err(|_| BridgeFailure::PythonUnavailable)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request.to_string().as_bytes())
            .map_err(|_| BridgeFailure::BridgeUnavailable)?;
    }
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(timeout_secs.saturating_add(2))) {
        Ok(Ok(output)) => {
            if !output.status.success() {
                return Err(BridgeFailure::Unavailable);
            }
            String::from_utf8(output.stdout).map_err(|_| BridgeFailure::Malformed)
        }
        Ok(Err(_)) => Err(BridgeFailure::PythonUnavailable),
        Err(_) => {
            kill_pid(pid);
            Err(BridgeFailure::Timeout)
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
}

fn python_exe(cfg: &ProviderConfig) -> &str {
    if cfg.pkc_python_executable.trim().is_empty() {
        "python"
    } else {
        cfg.pkc_python_executable.trim()
    }
}

fn python_available(exe: &str) -> bool {
    Command::new(exe)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn existing_path_from_env(name: &str, must_be_file: bool) -> String {
    let Ok(value) = std::env::var(name) else {
        return String::new();
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let path = PathBuf::from(trimmed);
    let ok = if must_be_file {
        path.is_file()
    } else {
        path.is_dir()
    };
    if ok {
        path.to_string_lossy().into_owned()
    } else {
        String::new()
    }
}

fn discover_python() -> String {
    for candidate in ["python", "py", "python3"] {
        if python_available(candidate) {
            return candidate.to_string();
        }
    }
    String::new()
}

fn discover_source_id(root: &Path) -> Option<String> {
    let register = root.join("06_exports/creative-source-disclosure-register-v0.1.json");
    let raw = std::fs::read_to_string(register).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let mut found = Vec::new();
    collect_source_ids(&value, &mut found);
    found.into_iter().find(|id| id.starts_with("SRC-SHA256-"))
}

fn collect_source_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if s.starts_with("SRC-SHA256-") => out.push(s.clone()),
        Value::Array(items) => {
            for item in items {
                collect_source_ids(item, out);
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                collect_source_ids(v, out);
            }
        }
        _ => {}
    }
}

fn parse_health_state(raw: &str) -> Option<PkcHealthState> {
    match raw {
        "disabled" => Some(PkcHealthState::Disabled),
        "configured_not_tested" => Some(PkcHealthState::ConfiguredNotTested),
        "connecting" => Some(PkcHealthState::Connecting),
        "available_authorized" => Some(PkcHealthState::AvailableAuthorized),
        "available_unauthorized" => Some(PkcHealthState::AvailableUnauthorized),
        "pkc_unavailable" => Some(PkcHealthState::PkcUnavailable),
        "bridge_unavailable" => Some(PkcHealthState::BridgeUnavailable),
        "python_unavailable" => Some(PkcHealthState::PythonUnavailable),
        "misconfigured" => Some(PkcHealthState::Misconfigured),
        "local_model_unavailable" => Some(PkcHealthState::LocalModelUnavailable),
        "cloud_turn_skipped" => Some(PkcHealthState::CloudTurnSkipped),
        "retrieval_skipped" => Some(PkcHealthState::RetrievalSkipped),
        _ => None,
    }
}

fn is_greeting_or_thanks(lower: &str) -> bool {
    let compact = lower
        .trim_matches(|c: char| matches!(c, '!' | '?' | '.' | ','))
        .trim();
    matches!(
        compact,
        "hi" | "hello"
            | "hey"
            | "yo"
            | "sup"
            | "thanks"
            | "thank you"
            | "thx"
            | "good morning"
            | "good afternoon"
            | "good evening"
            | "good night"
            | "hello sammy"
            | "hi sammy"
            | "hey sammy"
    )
}

fn is_pure_arithmetic(lower: &str) -> bool {
    let stripped: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
    if stripped.is_empty() || !stripped.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    stripped.chars().all(|c| {
        c.is_ascii_digit() || matches!(c, '+' | '-' | '*' | '/' | '(' | ')' | '=' | '.')
    })
}

fn has_personal_knowledge_cue(lower: &str) -> bool {
    if lower.contains("what do you know about me")
        || lower.contains("do you remember")
        || lower.contains("tell me about my")
        || lower.contains("when did i")
        || lower.contains("where did i")
        || lower.contains("who is my")
        || lower.contains("what do i")
        || lower.contains("what have i")
    {
        return true;
    }
    if has_word(lower, "my") || has_word(lower, "mine") || has_word(lower, "myself") {
        return true;
    }
    let has_i = has_word(lower, "i")
        || has_word(lower, "i'm")
        || has_word(lower, "i've")
        || has_word(lower, "i'd");
    if !has_i {
        return false;
    }
    const EVENTS: &[&str] = &[
        "grow",
        "grew",
        "child",
        "family",
        "remember",
        "recall",
        "prefer",
        "preference",
        "decid",
        "lived",
        "project",
        "relationship",
        "used to",
        "documented",
        "history",
    ];
    EVENTS.iter().any(|cue| lower.contains(cue))
}

fn has_word(hay: &str, word: &str) -> bool {
    hay.split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .any(|w| w == word)
}

pub fn validate_or_error(cfg: &ProviderConfig) -> AppResult<()> {
    validate_settings(cfg).map_err(AppError::Config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorized_json(answer: &str) -> String {
        json!({
            "bridge_version": "1.0.0",
            "ok": true,
            "available": true,
            "payload": {
                "natural_answer": answer,
                "conversational_context": "",
                "authorized": true,
                "canonical_hash_verified": true,
                "protected_content_materialized": true
            }
        })
        .to_string()
    }

    #[test]
    fn authorized_payload_reaches_the_model_path() {
        let payload = conversational_payload_from_bridge_json(&authorized_json(
            "Dave grew up in Gloucester Township.",
        ));
        assert_eq!(
            payload.as_deref(),
            Some("Dave grew up in Gloucester Township.")
        );
    }

    #[test]
    fn unauthorized_payload_is_dropped() {
        let raw = json!({
            "bridge_version": "1.0.0",
            "ok": true,
            "available": true,
            "payload": {
                "natural_answer": "MUST_NOT_REACH_MODEL",
                "conversational_context": "",
                "authorized": false,
                "canonical_hash_verified": false,
                "protected_content_materialized": false,
                "safe_reason": "authorization_denied"
            }
        })
        .to_string();
        assert_eq!(conversational_payload_from_bridge_json(&raw), None);
    }

    #[test]
    fn unavailable_payload_is_dropped() {
        let raw = json!({
            "bridge_version": "1.0.0",
            "ok": false,
            "available": false,
            "payload": {
                "natural_answer": "",
                "authorized": false,
                "canonical_hash_verified": false,
                "protected_content_materialized": false,
                "safe_reason": "pkc_unavailable"
            }
        })
        .to_string();
        assert_eq!(conversational_payload_from_bridge_json(&raw), None);
    }

    #[test]
    fn bookkeeping_in_natural_answer_is_dropped() {
        assert_eq!(
            conversational_payload_from_bridge_json(&authorized_json(
                "authorization_denied CSDP-99"
            )),
            None
        );
    }

    #[test]
    fn disabled_config_does_not_query() {
        let cfg = ProviderConfig::default();
        assert!(!cfg.pkc_enabled);
        assert_eq!(
            authorized_payload_for_turn(&cfg, "Where did I grow up?"),
            None
        );
    }

    #[test]
    fn malformed_json_is_dropped() {
        assert_eq!(conversational_payload_from_bridge_json("not-json"), None);
    }

    #[test]
    fn empty_authorized_payload_is_dropped() {
        assert_eq!(
            conversational_payload_from_bridge_json(&authorized_json("")),
            None
        );
    }

    #[test]
    fn sanitization_strips_bookkeeping_and_keeps_classification() {
        let evidence =
            parse_sanitized(&authorized_json("The family moved often.")).expect("ok");
        assert_eq!(evidence.classification, EvidenceClass::StoredFact);
        assert_eq!(evidence.source_type, "external_pkc");
        assert!(evidence.authorized);
        assert!(!grounding::leaks_pkc_bookkeeping(&evidence.model_safe_text));
        let packed = package_model_safe_context(&evidence.model_safe_text);
        assert!(packed.contains("stored/source-backed fact"));
        assert!(packed.contains("You previously described"));
        assert!(packed.contains("That suggests"));
        assert!(!packed.contains("CSDP-"));
        assert!(!packed.contains("canonical_hash"));
    }

    #[test]
    fn retrieval_judgment_skips_greetings_math_and_generic_facts() {
        for q in [
            "hello",
            "hi sammy",
            "thanks",
            "2 + 2",
            "what is 12*8",
            "What is the capital of France?",
            "How do I write a for loop?",
        ] {
            assert!(!should_retrieve_for_turn(q), "{q}");
        }
    }

    #[test]
    fn retrieval_judgment_selects_personal_history_questions() {
        for q in [
            "Where did I grow up?",
            "What do you remember about my childhood?",
            "What are my preferences for tone?",
            "When did I decide to start this project?",
            "Tell me about my family.",
            "What do you know about me?",
        ] {
            assert!(should_retrieve_for_turn(q), "{q}");
        }
    }

    #[test]
    fn health_disabled_without_probe() {
        let report = health_check(&ProviderConfig::default(), false);
        assert_eq!(report.state, PkcHealthState::Disabled);
        assert!(!report.probed);
        assert!(report.local_only);
        assert_eq!(report.consumer, "sammy");
        assert_eq!(report.purpose, PURPOSE);
    }

    #[test]
    fn validate_rejects_enabled_without_paths() {
        let cfg = ProviderConfig {
            pkc_enabled: true,
            ..ProviderConfig::default()
        };
        assert!(validate_settings(&cfg).is_err());
    }

    #[test]
    fn validate_allows_disabled_incomplete() {
        assert!(validate_settings(&ProviderConfig::default()).is_ok());
    }

    #[test]
    fn validate_rejects_relative_bridge() {
        let cfg = ProviderConfig {
            pkc_enabled: true,
            pkc_bridge_script: "bridge.py".into(),
            pkc_source_id: "SRC-SHA256-test".into(),
            ..ProviderConfig::default()
        };
        assert!(validate_settings(&cfg).is_err());
    }

    #[test]
    fn provenance_round_trips_without_bodies() {
        let conn = Connection::open_in_memory().unwrap();
        let view = PkcTurnView {
            used: true,
            state: PkcHealthState::AvailableAuthorized.as_str().into(),
            source_id: Some("SRC-SHA256-test".into()),
            classification: Some("stored_fact".into()),
            request_id: Some("req-1".into()),
            payload_sha256: Some("abc".into()),
            payload_chars: 12,
            authorized: Some(true),
            ..PkcTurnView::default()
        };
        store_provenance(&conn, "msg-1", &view).unwrap();
        let loaded = load_provenance(&conn, "msg-1").unwrap();
        assert!(loaded.used);
        assert_eq!(loaded.payload_sha256.as_deref(), Some("abc"));
        let dump = serde_json::to_string(&loaded).unwrap();
        assert!(!dump.contains("Gloucester"));
    }

    #[test]
    fn consult_skips_when_disabled() {
        let outcome =
            consult_for_local_turn(&ProviderConfig::default(), "Where did I grow up?");
        assert!(!outcome.used);
        assert_eq!(outcome.state, PkcHealthState::Disabled);
    }

    #[test]
    fn consult_skips_greetings_even_when_enabled() {
        let cfg = ProviderConfig {
            pkc_enabled: true,
            pkc_python_executable: "python-does-not-exist-xyz".into(),
            pkc_bridge_script: r"C:\missing\bridge.py".into(),
            pkc_source_id: "SRC-SHA256-test".into(),
            ..ProviderConfig::default()
        };
        let outcome = consult_for_local_turn(&cfg, "hello");
        assert!(!outcome.used);
        assert_eq!(outcome.state, PkcHealthState::RetrievalSkipped);
    }

    #[test]
    fn missing_bridge_does_not_panic() {
        let cfg = ProviderConfig {
            pkc_enabled: true,
            pkc_bridge_script: r"C:\missing\storykeeper_pkc_bridge.py".into(),
            pkc_source_id: "SRC-SHA256-test".into(),
            ..ProviderConfig::default()
        };
        let outcome = consult_for_local_turn(&cfg, "Where did I grow up?");
        assert!(!outcome.used);
        assert_eq!(outcome.state, PkcHealthState::BridgeUnavailable);
        assert!(outcome.owner_view.owner_notice.is_some());
    }

    #[test]
    fn discovery_exposes_consumer_and_purpose() {
        let d = discover_defaults();
        assert_eq!(d.consumer, "sammy");
        assert_eq!(d.purpose, PURPOSE);
    }

    #[test]
    fn production_discovery_source_has_no_developer_machine_paths() {
        let src = include_str!("external_pkc.rs");
        let production = src
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests");
        assert!(
            !production.contains("personal-knowledge-corpus-scaffold"),
            "do not bake the development PKC path into discovery"
        );
        assert!(
            !production.contains(r"D:\dev\StoryKeeper"),
            "do not bake the development bridge path into discovery"
        );
        assert!(!production.contains("ZCodeProject"));
    }

    #[test]
    fn live_sammy_bridge_retrieves_authorized_seven_when_present() {
        let script = Path::new(r"D:\dev\StoryKeeper\tools\storykeeper_pkc_bridge.py");
        let root =
            Path::new(r"F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus");
        if !script.is_file() || !root.is_dir() {
            return;
        }
        let cfg = ProviderConfig {
            pkc_enabled: true,
            pkc_python_executable: "python".into(),
            pkc_bridge_script: script.to_string_lossy().into(),
            pkc_root: root.to_string_lossy().into(),
            pkc_source_id:
                "SRC-SHA256-9c53f4f121815dfd736e0377fafade3e649b2571f8858eebc8f1a92db2b0ec7f"
                    .into(),
            ..ProviderConfig::default()
        };
        let payload = authorized_payload_for_turn(
            &cfg,
            "What do you remember about moving as a child?",
        );
        let text = payload.expect("authorized Sammy retrieval should return a payload");
        assert!(!crate::grounding::leaks_pkc_bookkeeping(&text));
        assert!(text.len() > 20);
        assert!(!text.contains("CSDP-"));
        assert!(!text.contains("authorization_denied"));
        assert!(!text.contains("evidence_bookkeeping"));
    }
}
