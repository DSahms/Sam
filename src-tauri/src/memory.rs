//! `memory` — auditable memory candidates (Phase 7).
//!
//! Directive §26. Conversation content must NOT become durable memory
//! automatically. AI may propose memory candidates; the owner reviews each one
//! (approve / edit-and-approve / reject / defer / mark temporary / correct /
//! supersede / delete) before it can affect retrieval. Rejected memories never
//! appear in retrieval; deferred memories remain candidates, not facts.
//!
//! PKC retrieval never creates candidates. Only an explicit owner keep action
//! may enter this queue, with body-free PKC provenance attached. Approved
//! PKC-derived records are `local_only` and do not rewrite PKC.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::ids::{ConversationId, MemoryCandidateId, MessageId};

/// The review state of a memory candidate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateState {
    /// Awaiting owner review (the default).
    Pending,
    Approved,
    Rejected,
    Deferred,
    Temporary,
}

impl CandidateState {
    pub fn as_str(self) -> &'static str {
        match self {
            CandidateState::Pending => "pending",
            CandidateState::Approved => "approved",
            CandidateState::Rejected => "rejected",
            CandidateState::Deferred => "deferred",
            CandidateState::Temporary => "temporary",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(CandidateState::Pending),
            "approved" => Some(CandidateState::Approved),
            "rejected" => Some(CandidateState::Rejected),
            "deferred" => Some(CandidateState::Deferred),
            "temporary" => Some(CandidateState::Temporary),
            _ => None,
        }
    }

    /// Whether a candidate in this state may appear in retrieval. Only approved
    /// candidates (which become knowledge records) do; rejected never do.
    pub fn is_retrievable(self) -> bool {
        matches!(self, CandidateState::Approved)
    }
}

/// A memory candidate proposed by the AI or the owner, awaiting review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub candidate_id: String,
    pub proposed_text: String,
    pub record_type: String,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub reason: String,
    pub confidence: f64,
    pub sensitivity: String,
    pub suggested_domain: String,
    pub suggested_retention: String,
    pub provider_used: String,
    pub crossed_to_cloud: bool,
    pub created_at: String,
    pub state: String,
    #[serde(default = "origin_conversation")]
    pub origin: String,
    pub epistemic_class: Option<String>,
    pub pkc_source_id: Option<String>,
    pub pkc_request_id: Option<String>,
    pub payload_sha256: Option<String>,
    #[serde(default)]
    pub payload_chars: u32,
    pub original_proposed_text: Option<String>,
    #[serde(default)]
    pub owner_edited: bool,
    pub conflict_record_id: Option<String>,
}

fn origin_conversation() -> String {
    "conversation".into()
}

/// Result of an explicit owner keep action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepResult {
    pub candidate_id: String,
    pub created: bool,
    pub duplicate: bool,
    pub already_durable: bool,
    pub conflict_record_id: Option<String>,
}

/// Propose a new memory candidate (the AI path). The candidate starts Pending
/// and does NOT affect retrieval until approved.
#[allow(clippy::too_many_arguments)]
pub fn propose(
    conn: &Connection,
    proposed_text: &str,
    record_type: &str,
    source_conversation_id: Option<ConversationId>,
    source_message_id: Option<MessageId>,
    reason: &str,
    confidence: f64,
    sensitivity: &str,
    suggested_domain: &str,
    provider_used: &str,
    crossed_to_cloud: bool,
) -> AppResult<MemoryCandidateId> {
    ensure_schema(conn)?;
    let id = MemoryCandidateId::new();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO memory_candidates(
            candidate_id, proposed_text, record_type, source_conversation_id,
            source_message_id, reason, confidence, sensitivity, suggested_domain,
            provider_used, crossed_to_cloud, created_at, state)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'pending')",
        params![
            id.to_string(),
            proposed_text,
            record_type,
            source_conversation_id.map(|c| c.to_string()),
            source_message_id.map(|m| m.to_string()),
            reason,
            confidence,
            sensitivity,
            suggested_domain,
            provider_used,
            crossed_to_cloud,
            now,
        ],
    )?;
    Ok(id)
}

/// Ensure the memory_candidates table exists (idempotent).
pub fn ensure_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_candidates (
            candidate_id           TEXT PRIMARY KEY,
            proposed_text          TEXT NOT NULL,
            record_type            TEXT NOT NULL,
            source_conversation_id TEXT,
            source_message_id      TEXT,
            reason                 TEXT NOT NULL DEFAULT '',
            confidence             REAL NOT NULL DEFAULT 0.0,
            sensitivity            TEXT NOT NULL DEFAULT 'normal',
            suggested_domain       TEXT NOT NULL DEFAULT '',
            suggested_retention    TEXT NOT NULL DEFAULT '',
            provider_used          TEXT NOT NULL DEFAULT '',
            crossed_to_cloud       INTEGER NOT NULL DEFAULT 0,
            created_at             TEXT NOT NULL,
            state                  TEXT NOT NULL DEFAULT 'pending',
            reviewed_at            TEXT,
            knowledge_record_id    TEXT
        );
         CREATE INDEX IF NOT EXISTS mc_state ON memory_candidates(state);
         CREATE TABLE IF NOT EXISTS memory_candidate_provenance (
            candidate_id TEXT PRIMARY KEY,
            origin TEXT NOT NULL DEFAULT 'conversation',
            epistemic_class TEXT,
            pkc_source_id TEXT,
            pkc_request_id TEXT,
            payload_sha256 TEXT,
            payload_chars INTEGER NOT NULL DEFAULT 0,
            original_proposed_text TEXT NOT NULL DEFAULT '',
            owner_edited INTEGER NOT NULL DEFAULT 0,
            conflict_record_id TEXT,
            created_at TEXT NOT NULL
         );",
    )?;
    Ok(())
}

fn empty_candidate_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryCandidate> {
    Ok(MemoryCandidate {
        candidate_id: r.get::<_, String>(0)?,
        proposed_text: r.get::<_, String>(1)?,
        record_type: r.get::<_, String>(2)?,
        source_conversation_id: r.get::<_, Option<String>>(3)?,
        source_message_id: r.get::<_, Option<String>>(4)?,
        reason: r.get::<_, String>(5)?,
        confidence: r.get::<_, f64>(6)?,
        sensitivity: r.get::<_, String>(7)?,
        suggested_domain: r.get::<_, String>(8)?,
        suggested_retention: r.get::<_, String>(9)?,
        provider_used: r.get::<_, String>(10)?,
        crossed_to_cloud: r.get::<_, i64>(11)? != 0,
        created_at: r.get::<_, String>(12)?,
        state: r.get::<_, String>(13)?,
        origin: origin_conversation(),
        epistemic_class: None,
        pkc_source_id: None,
        pkc_request_id: None,
        payload_sha256: None,
        payload_chars: 0,
        original_proposed_text: None,
        owner_edited: false,
        conflict_record_id: None,
    })
}

/// List candidates, optionally filtered by state.
pub fn list(
    conn: &Connection,
    state_filter: Option<CandidateState>,
) -> AppResult<Vec<MemoryCandidate>> {
    ensure_schema(conn)?;
    let sql = match state_filter {
        Some(s) => format!(
            "SELECT candidate_id, proposed_text, record_type, source_conversation_id,
                    source_message_id, reason, confidence, sensitivity, suggested_domain,
                    suggested_retention, provider_used, crossed_to_cloud, created_at, state
             FROM memory_candidates WHERE state='{}' ORDER BY created_at DESC",
            s.as_str()
        ),
        None => "SELECT candidate_id, proposed_text, record_type, source_conversation_id,
                    source_message_id, reason, confidence, sensitivity, suggested_domain,
                    suggested_retention, provider_used, crossed_to_cloud, created_at, state
             FROM memory_candidates ORDER BY created_at DESC"
            .to_string(),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], empty_candidate_row)?;
    let mut collected = Vec::new();
    for r in rows {
        collected.push(r?);
    }
    drop(stmt);
    Ok(collected
        .into_iter()
        .map(|c| hydrate_provenance(conn, c))
        .collect())
}

/// Approve a candidate: promotes it to an approved knowledge record and
/// marks the candidate approved. Returns the new knowledge record id.
pub fn approve(
    conn: &Connection,
    vault_id: &str,
    candidate_id: MemoryCandidateId,
    edited_text: Option<&str>,
) -> AppResult<String> {
    let cand = get(conn, candidate_id)?;
    let original = cand
        .original_proposed_text
        .clone()
        .unwrap_or_else(|| cand.proposed_text.clone());
    let text = edited_text.unwrap_or(&cand.proposed_text);
    let owner_edited = normalize_text(text) != normalize_text(&original);
    let pkc_origin = cand.origin == "external_pkc";
    let record_type = record_type_for_class(cand.epistemic_class.as_deref());
    let sensitivity = if pkc_origin {
        crate::knowledge::Sensitivity::LocalOnly
    } else {
        crate::knowledge::Sensitivity::Normal
    };
    let mut record = crate::knowledge::NewRecord::approved_fact(text);
    record.record_type = record_type;
    record.sensitivity = sensitivity;
    if let Some(src) = cand.pkc_source_id.as_ref() {
        if !src.trim().is_empty() {
            record.source_ids = vec![src.clone()];
        }
    }
    let kid = crate::knowledge::create(conn, vault_id, &record)?;
    let provenance = serde_json::json!({
        "origin": cand.origin,
        "epistemic_class": cand.epistemic_class,
        "pkc_source_id": cand.pkc_source_id,
        "pkc_request_id": cand.pkc_request_id,
        "payload_sha256": cand.payload_sha256,
        "payload_chars": cand.payload_chars,
        "original_proposed_text": original,
        "owner_edited": owner_edited,
        "candidate_id": cand.candidate_id,
        "note": "Sammy durable memory from owner review; does not rewrite PKC",
    });
    crate::knowledge::attach_review_metadata(
        conn,
        kid,
        &record.source_ids,
        &provenance,
        sensitivity,
    )?;
    if owner_edited {
        mark_owner_edited(conn, &cand.candidate_id)?;
    }
    if let Some(other) = cand.conflict_record_id.as_deref() {
        if let Ok(oid) = crate::ids::RecordId::parse(other) {
            let _ = crate::knowledge::mark_contradiction(conn, kid, oid);
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE memory_candidates SET state='approved', reviewed_at=?1, knowledge_record_id=?2,
                proposed_text=?3
         WHERE candidate_id=?4",
        params![now, kid.to_string(), text, candidate_id.to_string()],
    )?;
    maybe_audit(
        conn,
        "memory_candidate_approved",
        &serde_json::json!({
            "candidate_id": cand.candidate_id,
            "knowledge_record_id": kid.to_string(),
            "origin": cand.origin,
            "epistemic_class": cand.epistemic_class,
            "owner_edited": owner_edited,
            "sensitivity": sensitivity.as_str(),
            "pkc_source_id": cand.pkc_source_id,
            "payload_sha256": cand.payload_sha256,
        }),
    );
    Ok(kid.to_string())
}

/// Reject a candidate. It will never appear in retrieval.
pub fn reject(conn: &Connection, candidate_id: MemoryCandidateId) -> AppResult<()> {
    let origin = get(conn, candidate_id)
        .ok()
        .map(|c| c.origin)
        .unwrap_or_else(origin_conversation);
    set_state(conn, candidate_id, CandidateState::Rejected)?;
    maybe_audit(
        conn,
        "memory_candidate_rejected",
        &serde_json::json!({
            "candidate_id": candidate_id.to_string(),
            "origin": origin,
        }),
    );
    Ok(())
}

/// Defer a candidate. It remains a candidate, not a fact.
pub fn defer(conn: &Connection, candidate_id: MemoryCandidateId) -> AppResult<()> {
    set_state(conn, candidate_id, CandidateState::Deferred)
}

/// Mark a candidate temporary (session-only; not durable memory).
pub fn mark_temporary(
    conn: &Connection,
    candidate_id: MemoryCandidateId,
) -> AppResult<()> {
    set_state(conn, candidate_id, CandidateState::Temporary)
}

/// Delete a candidate entirely.
pub fn delete(conn: &Connection, candidate_id: MemoryCandidateId) -> AppResult<()> {
    ensure_schema(conn)?;
    conn.execute(
        "DELETE FROM memory_candidate_provenance WHERE candidate_id=?1",
        params![candidate_id.to_string()],
    )?;
    conn.execute(
        "DELETE FROM memory_candidates WHERE candidate_id=?1",
        params![candidate_id.to_string()],
    )?;
    Ok(())
}

/// Get a single candidate.
pub fn get(
    conn: &Connection,
    candidate_id: MemoryCandidateId,
) -> AppResult<MemoryCandidate> {
    ensure_schema(conn)?;
    let cand = {
        let mut stmt = conn.prepare(
            "SELECT candidate_id, proposed_text, record_type, source_conversation_id,
                    source_message_id, reason, confidence, sensitivity, suggested_domain,
                    suggested_retention, provider_used, crossed_to_cloud, created_at, state
             FROM memory_candidates WHERE candidate_id=?1",
        )?;
        stmt.query_row(params![candidate_id.to_string()], empty_candidate_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound("candidate".into())
                }
                other => AppError::from(other),
            })?
    };
    Ok(hydrate_provenance(conn, cand))
}

/// Explicit owner action: create a Memory Review candidate from a PKC-grounded
/// chat turn. Retrieval itself never calls this.
pub fn keep_from_pkc_turn(
    conn: &Connection,
    message_id: &str,
    proposed_text: &str,
    epistemic_class: &str,
) -> AppResult<KeepResult> {
    ensure_schema(conn)?;
    let text = normalize_text(proposed_text);
    if text.is_empty() {
        return Err(AppError::Config(
            "Write the statement you want to review before adding it.".into(),
        ));
    }
    if text.chars().count() > 2000 {
        return Err(AppError::Config(
            "Keep a short statement, not the whole source.".into(),
        ));
    }
    if crate::grounding::leaks_pkc_bookkeeping(&text) {
        return Err(AppError::Config(
            "That text still contains internal bookkeeping and cannot be saved.".into(),
        ));
    }
    let class = parse_epistemic_class(epistemic_class).ok_or_else(|| {
        AppError::Config(
            "Choose whether this is stored knowledge, something you described, or a suggestion."
                .into(),
        )
    })?;
    let pkc =
        crate::external_pkc::load_provenance(conn, message_id).ok_or_else(|| {
            AppError::Config("This answer did not use personal knowledge.".into())
        })?;
    if !pkc.used {
        return Err(AppError::Config(
            "This answer did not use personal knowledge.".into(),
        ));
    }
    let conversation_id: String = conn
        .query_row(
            "SELECT conversation_id FROM messages WHERE message_id=?1",
            params![message_id],
            |r| r.get(0),
        )
        .map_err(|_| AppError::NotFound("message".into()))?;

    if let Some(existing) = pending_for_message(conn, message_id)? {
        maybe_audit(
            conn,
            "pkc_memory_candidate_duplicate",
            &serde_json::json!({
                "candidate_id": existing,
                "message_id": message_id,
                "reason": "pending_for_message",
            }),
        );
        return Ok(KeepResult {
            candidate_id: existing,
            created: false,
            duplicate: true,
            already_durable: false,
            conflict_record_id: None,
        });
    }
    if let Some(kid) = durable_with_same_text(conn, &text)? {
        maybe_audit(
            conn,
            "pkc_memory_candidate_duplicate",
            &serde_json::json!({
                "knowledge_record_id": kid,
                "message_id": message_id,
                "reason": "already_durable",
            }),
        );
        return Ok(KeepResult {
            candidate_id: kid,
            created: false,
            duplicate: true,
            already_durable: true,
            conflict_record_id: None,
        });
    }
    if let Some(existing) = pending_with_same_text(conn, &text)? {
        maybe_audit(
            conn,
            "pkc_memory_candidate_duplicate",
            &serde_json::json!({
                "candidate_id": existing,
                "message_id": message_id,
                "reason": "pending_same_text",
            }),
        );
        return Ok(KeepResult {
            candidate_id: existing,
            created: false,
            duplicate: true,
            already_durable: false,
            conflict_record_id: None,
        });
    }

    let conflict = conflict_for_source(conn, pkc.source_id.as_deref(), &text)?;
    let conv = ConversationId::parse(&conversation_id)?;
    let msg = MessageId::parse(message_id)?;
    let record_type = record_type_for_class(Some(class)).as_str();
    let reason = match class {
        "testimony" => "Owner asked to review something they previously described.",
        "inference" => "Owner asked to review a suggestion, not stored fact.",
        _ => "Owner asked to review stored personal knowledge from PKC.",
    };
    let id = propose(
        conn,
        &text,
        record_type,
        Some(conv),
        Some(msg),
        reason,
        0.9,
        "local_only",
        "personal_knowledge",
        "pkc",
        false,
    )?;
    store_candidate_provenance(
        conn,
        &id.to_string(),
        "external_pkc",
        class,
        pkc.source_id.as_deref(),
        pkc.request_id.as_deref(),
        pkc.payload_sha256.as_deref(),
        pkc.payload_chars,
        &text,
        conflict.as_deref(),
    )?;
    maybe_audit(
        conn,
        "pkc_memory_candidate_proposed",
        &serde_json::json!({
            "candidate_id": id.to_string(),
            "message_id": message_id,
            "origin": "external_pkc",
            "epistemic_class": class,
            "pkc_source_id": pkc.source_id,
            "pkc_request_id": pkc.request_id,
            "payload_sha256": pkc.payload_sha256,
            "payload_chars": pkc.payload_chars,
            "conflict": conflict.is_some(),
            "created": true,
        }),
    );
    Ok(KeepResult {
        candidate_id: id.to_string(),
        created: true,
        duplicate: false,
        already_durable: false,
        conflict_record_id: conflict,
    })
}

pub fn parse_epistemic_class(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "stored_fact" | "fact" => Some("stored_fact"),
        "testimony" => Some("testimony"),
        "inference" => Some("inference"),
        _ => None,
    }
}

fn record_type_for_class(class: Option<&str>) -> crate::knowledge::RecordType {
    match class {
        Some("testimony") => crate::knowledge::RecordType::Quote,
        Some("inference") => crate::knowledge::RecordType::Conclusion,
        _ => crate::knowledge::RecordType::Fact,
    }
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hydrate_provenance(conn: &Connection, mut cand: MemoryCandidate) -> MemoryCandidate {
    let Ok(mut stmt) = conn.prepare(
        "SELECT origin, epistemic_class, pkc_source_id, pkc_request_id, payload_sha256,
                payload_chars, original_proposed_text, owner_edited, conflict_record_id
         FROM memory_candidate_provenance WHERE candidate_id=?1",
    ) else {
        return cand;
    };
    let row = stmt.query_row(params![cand.candidate_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, i64>(7)?,
            r.get::<_, Option<String>>(8)?,
        ))
    });
    if let Ok((origin, class, source, request, hash, chars, original, edited, conflict)) =
        row
    {
        cand.origin = origin;
        cand.epistemic_class = class;
        cand.pkc_source_id = source;
        cand.pkc_request_id = request;
        cand.payload_sha256 = hash;
        cand.payload_chars = chars as u32;
        cand.original_proposed_text = Some(original);
        cand.owner_edited = edited != 0;
        cand.conflict_record_id = conflict;
    }
    cand
}

#[allow(clippy::too_many_arguments)]
fn store_candidate_provenance(
    conn: &Connection,
    candidate_id: &str,
    origin: &str,
    class: &str,
    source_id: Option<&str>,
    request_id: Option<&str>,
    payload_sha256: Option<&str>,
    payload_chars: u32,
    original_text: &str,
    conflict_record_id: Option<&str>,
) -> AppResult<()> {
    ensure_schema(conn)?;
    conn.execute(
        "INSERT INTO memory_candidate_provenance(
            candidate_id, origin, epistemic_class, pkc_source_id, pkc_request_id,
            payload_sha256, payload_chars, original_proposed_text, owner_edited,
            conflict_record_id, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,?9,?10)",
        params![
            candidate_id,
            origin,
            class,
            source_id,
            request_id,
            payload_sha256,
            payload_chars as i64,
            original_text,
            conflict_record_id,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn mark_owner_edited(conn: &Connection, candidate_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE memory_candidate_provenance SET owner_edited=1 WHERE candidate_id=?1",
        params![candidate_id],
    )?;
    Ok(())
}

fn pending_for_message(conn: &Connection, message_id: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT candidate_id FROM memory_candidates
         WHERE source_message_id=?1 AND state IN ('pending','deferred')
         ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![message_id])?;
    Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
}

fn pending_with_same_text(conn: &Connection, text: &str) -> AppResult<Option<String>> {
    let pending = list(conn, Some(CandidateState::Pending))?;
    Ok(pending
        .into_iter()
        .find(|c| normalize_text(&c.proposed_text) == text)
        .map(|c| c.candidate_id))
}

fn durable_with_same_text(conn: &Connection, text: &str) -> AppResult<Option<String>> {
    if !knowledge_table_exists(conn)? {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT record_id, canonical_text FROM knowledge_records_full
         WHERE status IN ('approved','disputed')",
    )?;
    let rows =
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, canonical) = row?;
        if normalize_text(&canonical) == text {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn conflict_for_source(
    conn: &Connection,
    source_id: Option<&str>,
    text: &str,
) -> AppResult<Option<String>> {
    let Some(src) = source_id.filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    if !knowledge_table_exists(conn)? {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT record_id, canonical_text, source_ids FROM knowledge_records_full
         WHERE status IN ('approved','disputed')",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (id, canonical, source_ids) = row?;
        if source_ids.contains(src) && normalize_text(&canonical) != text {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn knowledge_table_exists(conn: &Connection) -> AppResult<bool> {
    let exists: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='knowledge_records_full'",
        [],
        |r| r.get(0),
    )?;
    Ok(exists > 0)
}

fn maybe_audit(conn: &Connection, action: &str, detail: &serde_json::Value) {
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='audit_events'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return;
    }
    let _ = crate::audit::record(
        conn,
        crate::audit::AuditCategory::Memory,
        action,
        None,
        detail,
    );
}

fn set_state(
    conn: &Connection,
    candidate_id: MemoryCandidateId,
    state: CandidateState,
) -> AppResult<()> {
    ensure_schema(conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE memory_candidates SET state=?1, reviewed_at=?2 WHERE candidate_id=?3",
        params![state.as_str(), now, candidate_id.to_string()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        ensure_schema(&c).unwrap();
        c.execute_batch(
            "CREATE TABLE knowledge_records_full (
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
                record_id UNINDEXED, canonical_text);
             CREATE TABLE audit_events (
                seq INTEGER PRIMARY KEY AUTOINCREMENT, event_id TEXT NOT NULL UNIQUE,
                occurred_at TEXT NOT NULL, actor TEXT, category TEXT NOT NULL,
                action TEXT NOT NULL, detail_json TEXT NOT NULL DEFAULT '{}');",
        )
        .unwrap();
        c
    }

    fn seed_pkc_message(conn: &Connection, used: bool) -> (String, String) {
        crate::conversation::ensure_schema(conn).unwrap();
        crate::external_pkc::ensure_provenance_schema(conn).unwrap();
        let conv = crate::conversation::create(conn, Some("c")).unwrap();
        crate::conversation::append_message(
            conn,
            conv,
            crate::conversation::Role::User,
            "Where did I grow up?",
        )
        .unwrap();
        let assistant = crate::conversation::append_message(
            conn,
            conv,
            crate::conversation::Role::Assistant,
            "You grew up in Gloucester Township.",
        )
        .unwrap();
        crate::external_pkc::store_provenance(
            conn,
            &assistant.to_string(),
            &crate::external_pkc::PkcTurnView {
                used,
                state: if used {
                    "available_authorized".into()
                } else {
                    "cloud_turn_skipped".into()
                },
                source_id: Some("SRC-SHA256-test".into()),
                classification: Some("stored_fact".into()),
                request_id: Some("req-keep".into()),
                payload_sha256: Some("abc123".into()),
                payload_chars: 40,
                authorized: Some(used),
                ..crate::external_pkc::PkcTurnView::default()
            },
        )
        .unwrap();
        (conv.to_string(), assistant.to_string())
    }

    fn audit_blob(conn: &Connection) -> String {
        conn.prepare("SELECT group_concat(detail_json, '\n') FROM audit_events")
            .unwrap()
            .query_row([], |r| r.get::<_, Option<String>>(0))
            .unwrap()
            .unwrap_or_default()
    }

    #[test]
    fn propose_creates_pending_candidate() {
        let c = fresh_conn();
        let id = propose(
            &c,
            "the owner likes tea",
            "preference",
            None,
            None,
            "user said so",
            0.8,
            "normal",
            "preferences",
            "mock",
            false,
        )
        .unwrap();
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "pending");
        assert_eq!(cand.proposed_text, "the owner likes tea");
        assert!(!cand.crossed_to_cloud);
    }

    #[test]
    fn approve_promotes_to_knowledge_record() {
        let c = fresh_conn();
        let id = propose(
            &c,
            "likes coffee",
            "preference",
            None,
            None,
            "",
            0.5,
            "normal",
            "",
            "mock",
            false,
        )
        .unwrap();
        let kid = approve(&c, "v1", id, None).unwrap();
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "approved");
        let count: i64 = c
            .query_row(
                "SELECT count(*) FROM knowledge_records_full WHERE record_id=?1 AND status='approved'",
                params![kid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn approve_with_edited_text_uses_edit() {
        let c = fresh_conn();
        let id = propose(
            &c, "original", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        let kid = approve(&c, "v1", id, Some("edited text")).unwrap();
        let text: String = c
            .query_row(
                "SELECT canonical_text FROM knowledge_records_full WHERE record_id=?1",
                params![kid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "edited text");
    }

    #[test]
    fn rejected_candidate_does_not_become_knowledge() {
        let c = fresh_conn();
        let id = propose(
            &c,
            "rejected thing",
            "fact",
            None,
            None,
            "",
            0.5,
            "normal",
            "",
            "mock",
            false,
        )
        .unwrap();
        reject(&c, id).unwrap();
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "rejected");
        let count: i64 = c
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn deferred_remains_candidate() {
        let c = fresh_conn();
        let id = propose(
            &c, "defer me", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        defer(&c, id).unwrap();
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "deferred");
        let count: i64 = c
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn list_filters_by_state() {
        let c = fresh_conn();
        let a = propose(
            &c, "a", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        let b = propose(
            &c, "b", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        reject(&c, a).unwrap();
        let pending = list(&c, Some(CandidateState::Pending)).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].candidate_id, b.to_string());
        let rejected = list(&c, Some(CandidateState::Rejected)).unwrap();
        assert_eq!(rejected.len(), 1);
    }

    #[test]
    fn delete_removes_candidate() {
        let c = fresh_conn();
        let id = propose(
            &c, "gone", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        delete(&c, id).unwrap();
        assert!(get(&c, id).is_err());
    }

    #[test]
    fn mark_temporary_sets_state() {
        let c = fresh_conn();
        let id = propose(
            &c, "temp", "fact", None, None, "", 0.5, "normal", "", "mock", false,
        )
        .unwrap();
        mark_temporary(&c, id).unwrap();
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "temporary");
    }

    #[test]
    fn keep_from_pkc_creates_candidate_not_durable_memory() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let result =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        assert!(result.created);
        let cand =
            get(&c, MemoryCandidateId::parse(&result.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.state, "pending");
        assert_eq!(cand.origin, "external_pkc");
        assert_eq!(cand.epistemic_class.as_deref(), Some("stored_fact"));
        assert_eq!(cand.pkc_source_id.as_deref(), Some("SRC-SHA256-test"));
        let knowledge: i64 = c
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(knowledge, 0);
        assert!(crate::audit::assert_action_recorded(
            &c,
            "pkc_memory_candidate_proposed"
        ));
        let blob = audit_blob(&c);
        assert!(!blob.contains("Gloucester"));
        assert!(!blob.contains("Grew up"));
    }

    #[test]
    fn keep_requires_pkc_grounded_turn() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, false);
        let err = keep_from_pkc_turn(&c, &msg, "secret", "stored_fact").unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
        assert!(list(&c, None).unwrap().is_empty());
    }

    #[test]
    fn keep_retains_testimony_and_inference_classes() {
        let c = fresh_conn();
        let (conv, msg) = seed_pkc_message(&c, true);
        let t = keep_from_pkc_turn(&c, &msg, "You described moving often", "testimony")
            .unwrap();
        let cand = get(&c, MemoryCandidateId::parse(&t.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.epistemic_class.as_deref(), Some("testimony"));
        reject(&c, MemoryCandidateId::parse(&t.candidate_id).unwrap()).unwrap();
        let assistant2 = crate::conversation::append_message(
            &c,
            crate::ids::ConversationId::parse(&conv).unwrap(),
            crate::conversation::Role::Assistant,
            "That suggests the moves mattered.",
        )
        .unwrap();
        crate::external_pkc::store_provenance(
            &c,
            &assistant2.to_string(),
            &crate::external_pkc::PkcTurnView {
                used: true,
                state: "available_authorized".into(),
                source_id: Some("SRC-SHA256-test".into()),
                classification: Some("stored_fact".into()),
                payload_sha256: Some("abc123".into()),
                payload_chars: 40,
                authorized: Some(true),
                ..crate::external_pkc::PkcTurnView::default()
            },
        )
        .unwrap();
        let i = keep_from_pkc_turn(
            &c,
            &assistant2.to_string(),
            "The moves may have mattered",
            "inference",
        )
        .unwrap();
        let cand = get(&c, MemoryCandidateId::parse(&i.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.epistemic_class.as_deref(), Some("inference"));
        assert_eq!(cand.record_type, "conclusion");
    }

    #[test]
    fn repeated_keep_does_not_duplicate_pending_candidate() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let a =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        let b =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        assert!(a.created);
        assert!(!b.created && b.duplicate);
        assert_eq!(a.candidate_id, b.candidate_id);
        assert_eq!(list(&c, Some(CandidateState::Pending)).unwrap().len(), 1);
    }

    #[test]
    fn approve_pkc_candidate_writes_one_local_only_memory() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        let id = MemoryCandidateId::parse(&kept.candidate_id).unwrap();
        let kid = approve(&c, "v1", id, Some("Grew up in Gloucester Township.")).unwrap();
        let rec = crate::knowledge::get(&c, crate::ids::RecordId::parse(&kid).unwrap())
            .unwrap();
        assert_eq!(rec.status, "approved");
        assert_eq!(rec.sensitivity, "local_only");
        assert_eq!(rec.canonical_text, "Grew up in Gloucester Township.");
        assert!(rec.provenance["owner_edited"].as_bool().unwrap());
        assert_eq!(
            rec.provenance["original_proposed_text"].as_str().unwrap(),
            "Grew up in Gloucester Township"
        );
        assert_eq!(
            rec.provenance["epistemic_class"].as_str().unwrap(),
            "stored_fact"
        );
        assert!(!rec.provenance.to_string().contains("authorization_denied"));
        let count: i64 = c
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        let blob = audit_blob(&c);
        assert!(!blob.contains("Gloucester"));
    }

    #[test]
    fn reject_pkc_candidate_writes_no_durable_memory() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        reject(&c, MemoryCandidateId::parse(&kept.candidate_id).unwrap()).unwrap();
        let count: i64 = c
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn keep_surfaces_conflict_without_picking_a_side() {
        let c = fresh_conn();
        let mut existing =
            crate::knowledge::NewRecord::approved_fact("Grew up somewhere else");
        existing.source_ids = vec!["SRC-SHA256-test".into()];
        let old = crate::knowledge::create(&c, "v1", &existing).unwrap();
        crate::knowledge::attach_review_metadata(
            &c,
            old,
            &existing.source_ids,
            &serde_json::json!({"origin":"external_pkc"}),
            crate::knowledge::Sensitivity::LocalOnly,
        )
        .unwrap();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        assert_eq!(kept.conflict_record_id, Some(old.to_string()));
        let cand =
            get(&c, MemoryCandidateId::parse(&kept.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.state, "pending");
    }

    #[test]
    fn keep_skips_when_identical_durable_memory_exists() {
        let c = fresh_conn();
        crate::knowledge::create(
            &c,
            "v1",
            &crate::knowledge::NewRecord::approved_fact("Grew up in Gloucester Township"),
        )
        .unwrap();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        assert!(kept.already_durable);
        assert!(!kept.created);
        assert!(list(&c, Some(CandidateState::Pending)).unwrap().is_empty());
    }

    #[test]
    fn pending_pkc_candidate_survives_reopen() {
        let path = std::env::temp_dir()
            .join(format!("sammy-mem-restart-{}.db", uuid::Uuid::new_v4()));
        let id = {
            let c = Connection::open(&path).unwrap();
            ensure_schema(&c).unwrap();
            c.execute_batch(
                "CREATE TABLE knowledge_records_full (
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
                    schema_version INTEGER NOT NULL DEFAULT 1);",
            )
            .unwrap();
            let (_conv, msg) = seed_pkc_message(&c, true);
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap()
                .candidate_id
        };
        let c2 = Connection::open(&path).unwrap();
        let cand = get(&c2, MemoryCandidateId::parse(&id).unwrap()).unwrap();
        assert_eq!(cand.state, "pending");
        assert_eq!(cand.origin, "external_pkc");
        assert_eq!(cand.pkc_source_id.as_deref(), Some("SRC-SHA256-test"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pkc_offline_does_not_drop_existing_candidate() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        c.execute("DELETE FROM message_pkc_provenance", []).unwrap();
        let cand =
            get(&c, MemoryCandidateId::parse(&kept.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.origin, "external_pkc");
        assert_eq!(cand.state, "pending");
    }

    #[test]
    fn malformed_keep_text_is_rejected() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        assert!(keep_from_pkc_turn(&c, &msg, "", "stored_fact").is_err());
        assert!(keep_from_pkc_turn(&c, &msg, "ok", "not-a-class").is_err());
        assert!(keep_from_pkc_turn(
            &c,
            &msg,
            "authorization_denied CSDP-1",
            "stored_fact"
        )
        .is_err());
    }

    #[test]
    fn keep_without_source_id_still_creates_candidate() {
        let c = fresh_conn();
        crate::conversation::ensure_schema(&c).unwrap();
        crate::external_pkc::ensure_provenance_schema(&c).unwrap();
        let conv = crate::conversation::create(&c, Some("c")).unwrap();
        crate::conversation::append_message(
            &c,
            conv,
            crate::conversation::Role::User,
            "What did I say?",
        )
        .unwrap();
        let assistant = crate::conversation::append_message(
            &c,
            conv,
            crate::conversation::Role::Assistant,
            "You described moving often.",
        )
        .unwrap();
        crate::external_pkc::store_provenance(
            &c,
            &assistant.to_string(),
            &crate::external_pkc::PkcTurnView {
                used: true,
                state: "available_authorized".into(),
                source_id: None,
                classification: None,
                payload_chars: 12,
                authorized: Some(true),
                ..crate::external_pkc::PkcTurnView::default()
            },
        )
        .unwrap();
        let kept = keep_from_pkc_turn(
            &c,
            &assistant.to_string(),
            "You described moving often",
            "testimony",
        )
        .unwrap();
        let cand =
            get(&c, MemoryCandidateId::parse(&kept.candidate_id).unwrap()).unwrap();
        assert_eq!(cand.origin, "external_pkc");
        assert!(cand.pkc_source_id.is_none());
        assert_eq!(cand.epistemic_class.as_deref(), Some("testimony"));
    }

    #[test]
    fn inference_approval_does_not_become_fact() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "The moves may have mattered", "inference")
                .unwrap();
        let kid = approve(
            &c,
            "v1",
            MemoryCandidateId::parse(&kept.candidate_id).unwrap(),
            None,
        )
        .unwrap();
        let rec = crate::knowledge::get(&c, crate::ids::RecordId::parse(&kid).unwrap())
            .unwrap();
        assert_eq!(rec.record_type, "conclusion");
        assert_eq!(
            rec.provenance["epistemic_class"].as_str().unwrap(),
            "inference"
        );
        assert_eq!(rec.sensitivity, "local_only");
    }

    #[test]
    fn rejected_candidate_is_not_recreated_by_another_keep_on_same_message() {
        let c = fresh_conn();
        let (_conv, msg) = seed_pkc_message(&c, true);
        let kept =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        reject(&c, MemoryCandidateId::parse(&kept.candidate_id).unwrap()).unwrap();
        let again =
            keep_from_pkc_turn(&c, &msg, "Grew up in Gloucester Township", "stored_fact")
                .unwrap();
        assert!(again.created);
        assert_ne!(again.candidate_id, kept.candidate_id);
        assert_eq!(list(&c, Some(CandidateState::Pending)).unwrap().len(), 1);
        assert_eq!(list(&c, Some(CandidateState::Rejected)).unwrap().len(), 1);
    }
}
