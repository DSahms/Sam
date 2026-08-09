//! `memory` — auditable memory candidates (Phase 7).
//!
//! Directive §26. Conversation content must NOT become durable memory
//! automatically. AI may propose memory candidates; the owner reviews each one
//! (approve / edit-and-approve / reject / defer / mark temporary / correct /
//! supersede / delete) before it can affect retrieval. Rejected memories never
//! appear in retrieval; deferred memories remain candidates, not facts.
//!
//! Approved candidates become knowledge records (record_type = memory_candidate
//! → promoted to an approved fact/claim on approval). The candidate itself is
//! preserved with full provenance (source conversation/message id, provider
//! used, whether the source crossed to a cloud provider).

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

/// A memory candidate proposed by the AI, awaiting owner review.
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
         CREATE INDEX IF NOT EXISTS mc_state ON memory_candidates(state);",
    )?;
    Ok(())
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
    let rows = stmt.query_map([], |r| {
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
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Approve a candidate: promotes it to an approved knowledge record (fact) and
/// marks the candidate approved. Returns the new knowledge record id.
pub fn approve(
    conn: &Connection,
    vault_id: &str,
    candidate_id: MemoryCandidateId,
    edited_text: Option<&str>,
) -> AppResult<String> {
    let cand = get(conn, candidate_id)?;
    let text = edited_text.unwrap_or(&cand.proposed_text);
    let record = crate::knowledge::NewRecord::approved_fact(text);
    let kid = crate::knowledge::create(conn, vault_id, &record)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE memory_candidates SET state='approved', reviewed_at=?1, knowledge_record_id=?2
         WHERE candidate_id=?3",
        params![now, kid.to_string(), candidate_id.to_string()],
    )?;
    Ok(kid.to_string())
}

/// Reject a candidate. It will never appear in retrieval.
pub fn reject(conn: &Connection, candidate_id: MemoryCandidateId) -> AppResult<()> {
    set_state(conn, candidate_id, CandidateState::Rejected)
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
    let mut stmt = conn.prepare(
        "SELECT candidate_id, proposed_text, record_type, source_conversation_id,
                source_message_id, reason, confidence, sensitivity, suggested_domain,
                suggested_retention, provider_used, crossed_to_cloud, created_at, state
         FROM memory_candidates WHERE candidate_id=?1",
    )?;
    stmt.query_row(params![candidate_id.to_string()], |r| {
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
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("candidate".into()),
        other => AppError::from(other),
    })
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
                record_id UNINDEXED, canonical_text);",
        )
        .unwrap();
        c
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
        // The candidate is approved and linked to the knowledge record.
        let cand = get(&c, id).unwrap();
        assert_eq!(cand.state, "approved");
        // The knowledge record exists and is current.
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
}
