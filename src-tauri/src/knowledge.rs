//! `knowledge` — curated knowledge records (Phase 3).
//!
//! Directive §17/§18. The canonical knowledge store is SQLCipher (never a
//! vector index — directive §15). Records are versioned, source-aware, and
//! review-gated: AI may propose candidates, but the owner approves before a
//! record becomes current truth. Contradictions preserve both claims;
//! corrections and supersession preserve history.
//!
//! Record states (directive §17): Candidate, Approved, Rejected, Disputed,
//! Superseded, Archived, Tombstoned. Tombstoned records do not appear in
//! current retrieval (directive §19).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::ids::RecordId;

/// Directive §17 record types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordType {
    Fact,
    Claim,
    Conclusion,
    Preference,
    Person,
    Relationship,
    Event,
    Place,
    Project,
    Decision,
    Commitment,
    Routine,
    Skill,
    Belief,
    Quote,
    SourceSummary,
    MemoryCandidate,
    Correction,
}

impl RecordType {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordType::Fact => "fact",
            RecordType::Claim => "claim",
            RecordType::Conclusion => "conclusion",
            RecordType::Preference => "preference",
            RecordType::Person => "person",
            RecordType::Relationship => "relationship",
            RecordType::Event => "event",
            RecordType::Place => "place",
            RecordType::Project => "project",
            RecordType::Decision => "decision",
            RecordType::Commitment => "commitment",
            RecordType::Routine => "routine",
            RecordType::Skill => "skill",
            RecordType::Belief => "belief",
            RecordType::Quote => "quote",
            RecordType::SourceSummary => "source_summary",
            RecordType::MemoryCandidate => "memory_candidate",
            RecordType::Correction => "correction",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "fact" => Some(RecordType::Fact),
            "claim" => Some(RecordType::Claim),
            "conclusion" => Some(RecordType::Conclusion),
            "preference" => Some(RecordType::Preference),
            "person" => Some(RecordType::Person),
            "relationship" => Some(RecordType::Relationship),
            "event" => Some(RecordType::Event),
            "place" => Some(RecordType::Place),
            "project" => Some(RecordType::Project),
            "decision" => Some(RecordType::Decision),
            "commitment" => Some(RecordType::Commitment),
            "routine" => Some(RecordType::Routine),
            "skill" => Some(RecordType::Skill),
            "belief" => Some(RecordType::Belief),
            "quote" => Some(RecordType::Quote),
            "source_summary" => Some(RecordType::SourceSummary),
            "memory_candidate" => Some(RecordType::MemoryCandidate),
            "correction" => Some(RecordType::Correction),
            _ => None,
        }
    }
}

/// Directive §17 record states. Tombstoned/Superseded/Rejected/Archived records
/// do not appear in current-truth retrieval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Candidate,
    Approved,
    Rejected,
    Disputed,
    Superseded,
    Archived,
    Tombstoned,
}

impl RecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordStatus::Candidate => "candidate",
            RecordStatus::Approved => "approved",
            RecordStatus::Rejected => "rejected",
            RecordStatus::Disputed => "disputed",
            RecordStatus::Superseded => "superseded",
            RecordStatus::Archived => "archived",
            RecordStatus::Tombstoned => "tombstoned",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "candidate" => Some(RecordStatus::Candidate),
            "approved" => Some(RecordStatus::Approved),
            "rejected" => Some(RecordStatus::Rejected),
            "disputed" => Some(RecordStatus::Disputed),
            "superseded" => Some(RecordStatus::Superseded),
            "archived" => Some(RecordStatus::Archived),
            "tombstoned" => Some(RecordStatus::Tombstoned),
            _ => None,
        }
    }

    /// Whether a record in this status counts as "current truth" for retrieval.
    pub fn is_current(self) -> bool {
        matches!(self, RecordStatus::Approved | RecordStatus::Disputed)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Normal,
    Sensitive,
    LocalOnly,
}

impl Sensitivity {
    pub fn as_str(self) -> &'static str {
        match self {
            Sensitivity::Normal => "normal",
            Sensitivity::Sensitive => "sensitive",
            Sensitivity::LocalOnly => "local_only",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Sensitivity::Normal),
            "sensitive" => Some(Sensitivity::Sensitive),
            "local_only" => Some(Sensitivity::LocalOnly),
            _ => None,
        }
    }
}

/// A full knowledge record (directive §17). JSON-array/object fields are stored
/// as TEXT and (de)serialized via serde_json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub record_id: String,
    pub record_type: String,
    pub canonical_text: String,
    pub status: String,
    pub source_ids: Vec<String>,
    pub source_locations: Vec<String>,
    pub provenance: serde_json::Value,
    pub confidence: f64,
    pub sensitivity: String,
    pub permissions: Vec<String>,
    pub domain_tags: Vec<String>,
    pub routing_tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub review_state: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub contradiction_set: Option<String>,
}

/// Input for creating a new record. Status defaults to Candidate unless the
/// owner explicitly approves.
#[derive(Debug, Clone)]
pub struct NewRecord {
    pub record_type: RecordType,
    pub canonical_text: String,
    pub source_ids: Vec<String>,
    pub sensitivity: Sensitivity,
    pub confidence: f64,
    pub domain_tags: Vec<String>,
    pub initial_status: RecordStatus,
}

impl NewRecord {
    /// A candidate proposed by the AI (owner must approve). This is the default
    /// path: model output is never automatically canonical (directive §17).
    pub fn candidate(rt: RecordType, text: impl Into<String>) -> Self {
        Self {
            record_type: rt,
            canonical_text: text.into(),
            source_ids: vec![],
            sensitivity: Sensitivity::Normal,
            confidence: 0.0,
            domain_tags: vec![],
            initial_status: RecordStatus::Candidate,
        }
    }
    /// A fact the owner manually enters and approves directly.
    pub fn approved_fact(text: impl Into<String>) -> Self {
        Self {
            record_type: RecordType::Fact,
            canonical_text: text.into(),
            source_ids: vec![],
            sensitivity: Sensitivity::Normal,
            confidence: 1.0,
            domain_tags: vec![],
            initial_status: RecordStatus::Approved,
        }
    }
}

/// Attach source/provenance metadata after create. Used when a Memory Review
/// candidate is approved so PKC origin survives without storing corpus bodies.
pub fn attach_review_metadata(
    conn: &Connection,
    id: RecordId,
    source_ids: &[String],
    provenance: &serde_json::Value,
    sensitivity: Sensitivity,
) -> AppResult<()> {
    conn.execute(
        "UPDATE knowledge_records_full
         SET source_ids=?1, provenance=?2, sensitivity=?3
         WHERE record_id=?4",
        params![
            serde_json::to_string(source_ids)?,
            serde_json::to_string(provenance)?,
            sensitivity.as_str(),
            id.to_string(),
        ],
    )?;
    Ok(())
}

/// Create a record. Returns its id. Inserts into both the table and the FTS
/// index (current records only).
pub fn create(conn: &Connection, vault_id: &str, new: &NewRecord) -> AppResult<RecordId> {
    let id = RecordId::new();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO knowledge_records_full(
            record_id, vault_id, record_type, canonical_text, status,
            source_ids, source_locations, provenance, confidence, sensitivity,
            permissions, domain_tags, routing_tags, created_at, updated_at,
            valid_from, valid_to, supersedes, superseded_by, review_state,
            reviewed_by, reviewed_at, contradiction_set, schema_version
         ) VALUES (?1,?2,?3,?4,?5,?6,'[]','{}',?7,?8,'[]',?9,'[]',?10,?10,
                   NULL,NULL,NULL,NULL,?11,NULL,NULL,NULL,1)",
        params![
            id.to_string(),
            vault_id,
            new.record_type.as_str(),
            new.canonical_text,
            new.initial_status.as_str(),
            serde_json::to_string(&new.source_ids)?,
            new.confidence,
            new.sensitivity.as_str(),
            serde_json::to_string(&new.domain_tags)?,
            now,
            new.initial_status.as_str(),
        ],
    )?;
    // Index only current-truth records in FTS.
    if new.initial_status.is_current() {
        reindex_record(conn, &id.to_string(), &new.canonical_text)?;
    }
    Ok(id)
}

/// Get a single record by id.
pub fn get(conn: &Connection, id: RecordId) -> AppResult<KnowledgeRecord> {
    let mut stmt = conn.prepare(
        "SELECT record_id, record_type, canonical_text, status, source_ids,
                source_locations, provenance, confidence, sensitivity, permissions,
                domain_tags, routing_tags, created_at, updated_at, valid_from, valid_to,
                supersedes, superseded_by, review_state, reviewed_by, reviewed_at,
                contradiction_set
         FROM knowledge_records_full WHERE record_id = ?1",
    )?;
    let r = stmt
        .query_row(params![id.to_string()], decode_row)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("record".into()),
            other => AppError::from(other),
        })?;
    Ok(r)
}

/// List records, optionally filtered by status (current-truth by default).
pub fn list_current(conn: &Connection) -> AppResult<Vec<KnowledgeRecord>> {
    list_by_filter(conn, "status IN ('approved','disputed')")
}

/// List all records regardless of status (for the What I Know management view).
pub fn list_all(conn: &Connection) -> AppResult<Vec<KnowledgeRecord>> {
    list_by_filter(conn, "1=1")
}

fn list_by_filter(
    conn: &Connection,
    where_clause: &str,
) -> AppResult<Vec<KnowledgeRecord>> {
    let sql = format!(
        "SELECT record_id, record_type, canonical_text, status, source_ids,
                source_locations, provenance, confidence, sensitivity, permissions,
                domain_tags, routing_tags, created_at, updated_at, valid_from, valid_to,
                supersedes, superseded_by, review_state, reviewed_by, reviewed_at,
                contradiction_set
         FROM knowledge_records_full WHERE {where_clause}
         ORDER BY updated_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], decode_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn decode_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeRecord> {
    Ok(KnowledgeRecord {
        record_id: r.get::<_, String>(0)?,
        record_type: r.get::<_, String>(1)?,
        canonical_text: r.get::<_, String>(2)?,
        status: r.get::<_, String>(3)?,
        source_ids: serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or_default(),
        source_locations: serde_json::from_str(&r.get::<_, String>(5)?)
            .unwrap_or_default(),
        provenance: serde_json::from_str(&r.get::<_, String>(6)?).unwrap_or_default(),
        confidence: r.get::<_, f64>(7)?,
        sensitivity: r.get::<_, String>(8)?,
        permissions: serde_json::from_str(&r.get::<_, String>(9)?).unwrap_or_default(),
        domain_tags: serde_json::from_str(&r.get::<_, String>(10)?).unwrap_or_default(),
        routing_tags: serde_json::from_str(&r.get::<_, String>(11)?).unwrap_or_default(),
        created_at: r.get::<_, String>(12)?,
        updated_at: r.get::<_, String>(13)?,
        valid_from: r.get::<_, Option<String>>(14)?,
        valid_to: r.get::<_, Option<String>>(15)?,
        supersedes: r.get::<_, Option<String>>(16)?,
        superseded_by: r.get::<_, Option<String>>(17)?,
        review_state: r.get::<_, String>(18)?,
        reviewed_by: r.get::<_, Option<String>>(19)?,
        reviewed_at: r.get::<_, Option<String>>(20)?,
        contradiction_set: r.get::<_, Option<String>>(21)?,
    })
}

/// Approve a candidate. Moves it to Approved status (becomes current truth) and
/// adds it to the FTS index. The owner is the actor.
pub fn approve(conn: &Connection, id: RecordId, actor: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE knowledge_records_full
         SET status='approved', review_state='approved', reviewed_by=?1, reviewed_at=?2,
             updated_at=?2
         WHERE record_id=?3",
        params![actor, now, id.to_string()],
    )?;
    let rec = get(conn, id)?;
    reindex_record(conn, &id.to_string(), &rec.canonical_text)?;
    Ok(())
}

/// Reject a candidate. It no longer appears in current retrieval.
pub fn reject(conn: &Connection, id: RecordId, actor: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE knowledge_records_full
         SET status='rejected', review_state='rejected', reviewed_by=?1, reviewed_at=?2,
             updated_at=?2
         WHERE record_id=?3",
        params![actor, now, id.to_string()],
    )?;
    drop_from_fts(conn, &id.to_string())?;
    Ok(())
}

/// Tombstone a record (directive §19). Removes it from current views and
/// indexes; leaves a minimal tombstone (no private content in audit).
pub fn tombstone(conn: &Connection, id: RecordId) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE knowledge_records_full
         SET status='tombstoned', review_state='tombstoned', updated_at=?1
         WHERE record_id=?2",
        params![now, id.to_string()],
    )?;
    drop_from_fts(conn, &id.to_string())?;
    Ok(())
}

/// Link two records as a contradiction set (directive §18). Both remain
/// available; the owner may later approve one.
pub fn mark_contradiction(conn: &Connection, a: RecordId, b: RecordId) -> AppResult<()> {
    let set_id = format!("contra-{}", a);
    let now = chrono::Utc::now().to_rfc3339();
    for id in [a, b] {
        conn.execute(
            "UPDATE knowledge_records_full
             SET contradiction_set=?1, status='disputed', review_state='disputed',
                 updated_at=?2
             WHERE record_id=?3",
            params![set_id, now, id.to_string()],
        )?;
    }
    Ok(())
}

/// Create a corrected record that supersedes an existing one (directive §18).
/// The old record is marked Superseded (preserved in history, not current truth)
/// and the new record links back via `supersedes`.
pub fn correct(
    conn: &Connection,
    vault_id: &str,
    old_id: RecordId,
    new_text: impl Into<String>,
    actor: &str,
) -> AppResult<RecordId> {
    let now = chrono::Utc::now().to_rfc3339();
    let old = get(conn, old_id)?;
    let mut new = NewRecord::approved_fact(new_text);
    new.record_type = RecordType::parse(&old.record_type).unwrap_or(RecordType::Fact);
    new.sensitivity = Sensitivity::parse(&old.sensitivity).unwrap_or(Sensitivity::Normal);
    new.domain_tags = old.domain_tags.clone();
    let new_id = create(conn, vault_id, &new)?;
    // Link the new record to the old.
    conn.execute(
        "UPDATE knowledge_records_full SET supersedes=?1, updated_at=?2 WHERE record_id=?3",
        params![old_id.to_string(), now, new_id.to_string()],
    )?;
    // Mark the old as superseded; it remains in history but not current truth.
    conn.execute(
        "UPDATE knowledge_records_full
         SET status='superseded', superseded_by=?1, updated_at=?2
         WHERE record_id=?3",
        params![new_id.to_string(), now, old_id.to_string()],
    )?;
    drop_from_fts(conn, &old_id.to_string())?;
    let _ = actor; // audited by the caller if needed
    Ok(new_id)
}

/// Lexical search over current records via FTS5.
pub fn search_current(
    conn: &Connection,
    query: &str,
    limit: u32,
) -> AppResult<Vec<String>> {
    // Sanitize into a phrase query so special characters in user input are
    // treated literally, not as FTS5 operators.
    let sanitized = query.replace('"', "\"\"");
    let phrase = format!("\"{sanitized}\"");
    let mut stmt = conn.prepare(
        "SELECT k.record_id FROM knowledge_fts f
         JOIN knowledge_records_full k ON k.record_id = f.record_id
         WHERE knowledge_fts MATCH ?1 AND k.status IN ('approved','disputed')
         ORDER BY rank LIMIT ?2",
    )?;
    let rows =
        stmt.query_map(params![phrase.as_str(), limit], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn reindex_record(conn: &Connection, id: &str, text: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO knowledge_fts(rowid, record_id, canonical_text)
         SELECT rowid, ?1, ?2 FROM knowledge_records_full WHERE record_id=?1",
        params![id, text],
    )?;
    Ok(())
}

fn drop_from_fts(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM knowledge_fts WHERE record_id=?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
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
    fn approved_fact_is_current_and_searchable() {
        let c = fresh_conn();
        let id = create(&c, "v1", &NewRecord::approved_fact("The sky is blue")).unwrap();
        let rec = get(&c, id).unwrap();
        assert_eq!(rec.status, "approved");
        assert!(RecordStatus::parse(&rec.status).unwrap().is_current());
        let hits = search_current(&c, "sky", 10).unwrap();
        assert!(hits.contains(&id.to_string()));
    }

    #[test]
    fn candidate_is_not_current_truth() {
        let c = fresh_conn();
        let id = create(
            &c,
            "v1",
            &NewRecord::candidate(RecordType::Claim, "maybe aliens"),
        )
        .unwrap();
        let rec = get(&c, id).unwrap();
        assert_eq!(rec.status, "candidate");
        assert!(!RecordStatus::parse(&rec.status).unwrap().is_current());
        // Not in FTS until approved.
        assert!(search_current(&c, "aliens", 10).unwrap().is_empty());
    }

    #[test]
    fn approve_promotes_candidate_into_retrieval() {
        let c = fresh_conn();
        let id = create(
            &c,
            "v1",
            &NewRecord::candidate(RecordType::Claim, "pluto is a planet"),
        )
        .unwrap();
        approve(&c, id, "owner").unwrap();
        let hits = search_current(&c, "pluto", 10).unwrap();
        assert!(hits.contains(&id.to_string()));
    }

    #[test]
    fn reject_removes_from_retrieval() {
        let c = fresh_conn();
        let id = create(
            &c,
            "v1",
            &NewRecord::approved_fact("temporary fact about cats"),
        )
        .unwrap();
        reject(&c, id, "owner").unwrap();
        assert!(search_current(&c, "cats", 10).unwrap().is_empty());
    }

    #[test]
    fn tombstone_removes_from_retrieval_and_preserves_record() {
        let c = fresh_conn();
        let id = create(&c, "v1", &NewRecord::approved_fact("a fact to forget")).unwrap();
        tombstone(&c, id).unwrap();
        assert!(search_current(&c, "forget", 10).unwrap().is_empty());
        // Record still exists (tombstoned), not deleted.
        let rec = get(&c, id).unwrap();
        assert_eq!(rec.status, "tombstoned");
    }

    #[test]
    fn contradiction_keeps_both_as_disputed_current() {
        let c = fresh_conn();
        let a = create(
            &c,
            "v1",
            &NewRecord::approved_fact("the meeting is Tuesday"),
        )
        .unwrap();
        let b = create(
            &c,
            "v1",
            &NewRecord::approved_fact("the meeting is Wednesday"),
        )
        .unwrap();
        mark_contradiction(&c, a, b).unwrap();
        let ra = get(&c, a).unwrap();
        let rb = get(&c, b).unwrap();
        assert_eq!(ra.status, "disputed");
        assert_eq!(rb.status, "disputed");
        // Both remain searchable (disputed is current).
        assert!(search_current(&c, "Tuesday", 10)
            .unwrap()
            .contains(&a.to_string()));
        assert!(search_current(&c, "Wednesday", 10)
            .unwrap()
            .contains(&b.to_string()));
    }

    #[test]
    fn correction_supersedes_old_record() {
        let c = fresh_conn();
        let old = create(&c, "v1", &NewRecord::approved_fact("earth is flat")).unwrap();
        let new = correct(&c, "v1", old, "earth is an oblate spheroid", "owner").unwrap();
        let rold = get(&c, old).unwrap();
        let rnew = get(&c, new).unwrap();
        assert_eq!(rold.status, "superseded");
        assert_eq!(rold.superseded_by, Some(new.to_string()));
        assert_eq!(rnew.supersedes, Some(old.to_string()));
        // Old no longer current-truth; new is.
        assert!(search_current(&c, "flat", 10).unwrap().is_empty());
        assert!(search_current(&c, "spheroid", 10)
            .unwrap()
            .contains(&new.to_string()));
    }

    #[test]
    fn list_current_excludes_non_current() {
        let c = fresh_conn();
        create(&c, "v1", &NewRecord::approved_fact("approved fact")).unwrap();
        create(
            &c,
            "v1",
            &NewRecord::candidate(RecordType::Fact, "candidate fact"),
        )
        .unwrap();
        let cur = list_current(&c).unwrap();
        assert_eq!(cur.len(), 1);
        assert_eq!(cur[0].canonical_text, "approved fact");
    }

    #[test]
    fn list_all_includes_everything() {
        let c = fresh_conn();
        create(&c, "v1", &NewRecord::approved_fact("a")).unwrap();
        create(&c, "v1", &NewRecord::candidate(RecordType::Fact, "b")).unwrap();
        assert_eq!(list_all(&c).unwrap().len(), 2);
    }

    #[test]
    fn search_handles_special_characters_in_query() {
        // Regression: FTS5 treats hyphens/parens as operators; the search must
        // sanitize the query into a phrase so special chars match literally.
        let c = fresh_conn();
        let id = create(
            &c,
            "v1",
            &NewRecord::approved_fact("ticket PROJ-123 (urgent)"),
        )
        .unwrap();
        let hits = search_current(&c, "PROJ-123", 10).unwrap();
        assert!(hits.contains(&id.to_string()));
        let hits2 = search_current(&c, "(urgent)", 10).unwrap();
        assert!(hits2.contains(&id.to_string()));
    }
}
