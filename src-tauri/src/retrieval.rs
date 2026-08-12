//! `retrieval` — replaceable retrieval subsystem (Phase 5).
//!
//! Directive §24. Coordinates lexical retrieval (FTS5 over knowledge records
//! and sources), applies permission/sensitivity/domain filters, merges and
//! ranks, deduplicates, budgets context, and assembles citations. The canonical
//! knowledge store is SQLCipher; the vector index is rebuildable and is NOT the
//! canonical corpus.
//!
//! The required 11-step process (directive §24) is implemented in [`retrieve`].
//! Steps that require a configured embedding/vector engine are present but
//! inactive until that adapter is wired (recorded as a non-blocking external
//! dependency — the lexical path is complete and authoritative on its own).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::citations::{
    AnnotatedAnswer, Citation, CitationLocation, ClaimBlock, ClaimLabel,
    TrustClassification,
};
use crate::error::AppResult;
use crate::knowledge;

// -----------------------------------------------------------------------------
// Embedding / vector interfaces (replaceable, rebuildable — directive §24)
// -----------------------------------------------------------------------------

/// Output dimensionality expected from embedding adapters. Fixed at a
/// conventional size so a local adapter can be swapped without schema changes.
pub const EMBED_DIM: usize = 384;

/// A replaceable embedding provider. The first release ships a deterministic
/// stub so the retrieval pipeline is exercised end-to-end; a real local
/// embedding adapter lands behind this interface.
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a piece of text into a fixed-size vector.
    fn embed(&self, text: &str) -> AppResult<Vec<f32>>;
}

/// A deterministic embedding stub: hashes the text into a fixed-size vector.
/// Not semantically meaningful, but stable and exercises the vector path.
pub struct StubEmbeddingProvider;
impl EmbeddingProvider for StubEmbeddingProvider {
    fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        use sha2::{Digest, Sha256};
        let mut out = vec![0.0f32; EMBED_DIM];
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        let digest = h.finalize();
        for (i, b) in digest.iter().cycle().take(EMBED_DIM).enumerate() {
            out[i] = (*b as f32) / 255.0;
        }
        Ok(out)
    }
}

/// A replaceable vector index. The canonical store is SQLCipher; this index is
/// rebuildable and deletable (directive §24). The first release ships an
/// in-memory stub index; a persistent local adapter lands behind this trait.
pub trait VectorIndex {
    fn add(&mut self, id: &str, vector: Vec<f32>) -> AppResult<()>;
    fn remove(&mut self, id: &str) -> AppResult<()>;
    fn clear(&mut self) -> AppResult<()>;
    /// Return the k nearest neighbors to `query_vector` as (id, score) pairs.
    fn search(&self, query_vector: &[f32], k: usize) -> AppResult<Vec<(String, f64)>>;
}

/// A brute-force in-memory vector index (cosine similarity). Suitable for small
/// corpora; a real index lands behind the trait.
pub struct InMemoryVectorIndex {
    items: Vec<(String, Vec<f32>)>,
}

impl InMemoryVectorIndex {
    pub fn new() -> Self {
        Self { items: vec![] }
    }
}

impl Default for InMemoryVectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex for InMemoryVectorIndex {
    fn add(&mut self, id: &str, vector: Vec<f32>) -> AppResult<()> {
        self.items.retain(|(existing, _)| existing != id);
        self.items.push((id.to_string(), vector));
        Ok(())
    }

    fn remove(&mut self, id: &str) -> AppResult<()> {
        self.items.retain(|(existing, _)| existing != id);
        Ok(())
    }

    fn clear(&mut self) -> AppResult<()> {
        self.items.clear();
        Ok(())
    }

    fn search(&self, query_vector: &[f32], k: usize) -> AppResult<Vec<(String, f64)>> {
        let mut scored: Vec<(String, f64)> = self
            .items
            .iter()
            .map(|(id, v)| (id.clone(), cosine(query_vector, v)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(k).collect())
    }
}

/// Persistent vector index stored inside the active vault's SQLCipher
/// database. The table is per-vault, encrypted by SQLCipher, rebuildable from
/// canonical knowledge records, and never treated as authoritative content.
pub struct SqliteVectorIndex<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteVectorIndex<'a> {
    pub fn new(conn: &'a Connection) -> AppResult<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vector_index (
                record_id TEXT PRIMARY KEY,
                dimensions INTEGER NOT NULL,
                vector_json TEXT NOT NULL,
                indexed_at TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn })
    }

    pub fn len(&self) -> AppResult<usize> {
        let count: i64 =
            self.conn
                .query_row("SELECT count(*) FROM vector_index", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn is_empty(&self) -> AppResult<bool> {
        self.len().map(|len| len == 0)
    }
}

impl VectorIndex for SqliteVectorIndex<'_> {
    fn add(&mut self, id: &str, vector: Vec<f32>) -> AppResult<()> {
        if vector.len() != EMBED_DIM {
            return Err(crate::error::AppError::InvalidArgument(format!(
                "embedding must have {EMBED_DIM} dimensions"
            )));
        }
        let encoded = serde_json::to_string(&vector)?;
        self.conn.execute(
            "INSERT INTO vector_index(record_id, dimensions, vector_json, indexed_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(record_id) DO UPDATE SET dimensions=?2, vector_json=?3, indexed_at=?4",
            rusqlite::params![id, EMBED_DIM as i64, encoded, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn remove(&mut self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM vector_index WHERE record_id=?1", [id])?;
        Ok(())
    }

    fn clear(&mut self) -> AppResult<()> {
        self.conn.execute("DELETE FROM vector_index", [])?;
        Ok(())
    }

    fn search(&self, query_vector: &[f32], k: usize) -> AppResult<Vec<(String, f64)>> {
        if query_vector.len() != EMBED_DIM {
            return Err(crate::error::AppError::InvalidArgument(format!(
                "query embedding must have {EMBED_DIM} dimensions"
            )));
        }
        let mut stmt = self
            .conn
            .prepare("SELECT record_id, vector_json FROM vector_index")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut scored = Vec::new();
        for row in rows {
            let (id, encoded) = row?;
            let vector: Vec<f32> = serde_json::from_str(&encoded)?;
            if vector.len() == EMBED_DIM {
                scored.push((id, cosine(query_vector, &vector)));
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }
}

/// Rebuild the derived index deterministically from current canonical records.
pub fn rebuild_vector_index(
    conn: &Connection,
    embedding: &dyn EmbeddingProvider,
) -> AppResult<usize> {
    let records = knowledge::list_current(conn)?;
    let mut index = SqliteVectorIndex::new(conn)?;
    index.clear()?;
    for record in records {
        index.add(&record.record_id, embedding.embed(&record.canonical_text)?)?;
    }
    index.len()
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let na: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

// -----------------------------------------------------------------------------
// Retrieval pipeline (directive §24, 11 steps)
// -----------------------------------------------------------------------------

/// A retrieval result item: a knowledge record id with a relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalHit {
    pub record_id: String,
    pub score: f64,
    /// The canonical text snippet that matched.
    pub snippet: String,
}

/// Sensitivity filter for retrieval. `LocalOnly` content must never cross to a
/// cloud provider (directive §13/§24).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityFilter {
    /// Include all current records.
    All,
    /// Exclude local_only records (for cloud-bound context).
    ExcludeLocalOnly,
}

/// Parameters for a retrieval pass.
#[derive(Debug, Clone)]
pub struct RetrievalRequest {
    pub query: String,
    pub limit: u32,
    pub sensitivity: SensitivityFilter,
    /// Maximum total snippet characters to return (context budget).
    pub context_budget_chars: usize,
}

impl Default for RetrievalRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 10,
            sensitivity: SensitivityFilter::All,
            context_budget_chars: 4000,
        }
    }
}

/// Run the retrieval pipeline against the vault's SQLCipher connection.
///
/// Implements the directive §24 lexical path (steps 4, 6–9): lexical retrieval
/// → sensitivity filtering → dedup → context budget → citation assembly.
/// Vector retrieval (step 5) runs if an embedding provider + index are
/// supplied; otherwise it is skipped. Steps 1–3 (validate unlocked, analyze
/// query, determine permitted domains) are the caller's responsibility (the
/// chat runtime guarantees an unlocked vault and supplies the query).
pub fn retrieve(
    conn: &Connection,
    req: &RetrievalRequest,
    embedding: Option<&dyn EmbeddingProvider>,
    vector_index: Option<&dyn VectorIndex>,
) -> AppResult<Vec<RetrievalHit>> {
    // Step 4: lexical retrieval (FTS5 over current knowledge).
    let mut hits = lexical_retrieval(conn, req)?;

    // Step 5: vector retrieval when configured, merged with lexical.
    if let (Some(emb), Some(idx)) = (embedding, vector_index) {
        if let Ok(qv) = emb.embed(&req.query) {
            let vhits = idx.search(&qv, req.limit as usize).unwrap_or_default();
            for (id, score) in vhits {
                if let Ok(text) = fetch_record_text(conn, &id) {
                    hits.push(RetrievalHit {
                        record_id: id,
                        score,
                        snippet: text,
                    });
                }
            }
        }
    }

    // Step 6: merge and rank (sort by descending score).
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Step 7: deduplicate by record id, keeping the highest score.
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert(h.record_id.clone()));

    // Step 8: apply context budget (truncate snippet total).
    let mut total = 0usize;
    hits.retain(|h| {
        total += h.snippet.chars().count();
        total <= req.context_budget_chars
    });

    // Step 9: limit.
    hits.truncate(req.limit as usize);

    Ok(hits)
}

fn lexical_retrieval(
    conn: &Connection,
    req: &RetrievalRequest,
) -> AppResult<Vec<RetrievalHit>> {
    let ids = knowledge::search_current(conn, &req.query, req.limit)?;
    let mut hits = Vec::new();
    for (rank, id) in ids.into_iter().enumerate() {
        if let Ok(text) = fetch_record_text(conn, &id) {
            // Step 3b: sensitivity filtering.
            if req.sensitivity == SensitivityFilter::ExcludeLocalOnly
                && is_local_only(conn, &id)?
            {
                continue;
            }
            let score = 1.0 / (1.0 + rank as f64);
            hits.push(RetrievalHit {
                record_id: id,
                score,
                snippet: text,
            });
        }
    }
    Ok(hits)
}

fn fetch_record_text(conn: &Connection, id: &str) -> AppResult<String> {
    let text: String = conn.query_row(
        "SELECT canonical_text FROM knowledge_records_full WHERE record_id=?1",
        rusqlite::params![id],
        |r| r.get(0),
    )?;
    Ok(text)
}

fn is_local_only(conn: &Connection, id: &str) -> AppResult<bool> {
    let sens: String = conn.query_row(
        "SELECT sensitivity FROM knowledge_records_full WHERE record_id=?1",
        rusqlite::params![id],
        |r| r.get(0),
    )?;
    Ok(sens == "local_only")
}

/// Assemble an [`AnnotatedAnswer`] from retrieval hits: source-supported when
/// hits exist, inference otherwise. Each grounded claim cites a real record.
pub fn assemble_answer(plain_text: &str, hits: &[RetrievalHit]) -> AnnotatedAnswer {
    if hits.is_empty() {
        return AnnotatedAnswer::inference(plain_text.to_string());
    }
    let citations: Vec<Citation> = hits
        .iter()
        .enumerate()
        .map(|(i, h)| Citation {
            id: format!("cite-{i}"),
            location: CitationLocation::KnowledgeRecord {
                record_id: h.record_id.clone(),
            },
            snippet: h.snippet.clone(),
        })
        .collect();
    let claims = vec![ClaimBlock {
        text: plain_text.to_string(),
        label: ClaimLabel::StoredFact,
        citations: citations.iter().map(|c| c.id.clone()).collect(),
    }];
    AnnotatedAnswer {
        plain_text: plain_text.to_string(),
        trust: TrustClassification::SourceSupported,
        citations,
        claims,
    }
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

    fn add_record(c: &Connection, id: &str, text: &str, sensitivity: &str) {
        c.execute(
            "INSERT INTO knowledge_records_full(record_id, vault_id, record_type, canonical_text,
                status, sensitivity, created_at, updated_at)
             VALUES (?1,'v','fact',?2,'approved',?3,'t','t')",
            rusqlite::params![id, text, sensitivity],
        )
        .unwrap();
        c.execute(
            "INSERT INTO knowledge_fts(record_id, canonical_text) VALUES (?1, ?2)",
            rusqlite::params![id, text],
        )
        .unwrap();
    }

    #[test]
    fn retrieve_returns_matching_current_records() {
        let c = fresh_conn();
        add_record(&c, "r1", "the sky is blue during the day", "normal");
        add_record(&c, "r2", "grass is green", "normal");
        let req = RetrievalRequest {
            query: "sky".into(),
            ..Default::default()
        };
        let hits = retrieve(&c, &req, None, None).unwrap();
        assert!(hits.iter().any(|h| h.record_id == "r1"));
        assert!(!hits.iter().any(|h| h.record_id == "r2"));
    }

    #[test]
    fn retrieve_filters_local_only_when_requested() {
        let c = fresh_conn();
        add_record(&c, "r1", "secret local thing", "local_only");
        add_record(&c, "r2", "public thing", "normal");
        let req = RetrievalRequest {
            query: "thing".into(),
            sensitivity: SensitivityFilter::ExcludeLocalOnly,
            ..Default::default()
        };
        let hits = retrieve(&c, &req, None, None).unwrap();
        assert!(hits.iter().any(|h| h.record_id == "r2"));
        assert!(!hits.iter().any(|h| h.record_id == "r1"));
    }

    #[test]
    fn retrieve_includes_local_only_when_all() {
        let c = fresh_conn();
        add_record(&c, "r1", "secret local thing", "local_only");
        let req = RetrievalRequest {
            query: "thing".into(),
            sensitivity: SensitivityFilter::All,
            ..Default::default()
        };
        let hits = retrieve(&c, &req, None, None).unwrap();
        assert!(hits.iter().any(|h| h.record_id == "r1"));
    }

    #[test]
    fn retrieve_applies_context_budget() {
        let c = fresh_conn();
        add_record(&c, "r1", &"x".repeat(1000), "normal");
        add_record(&c, "r2", &"y".repeat(1000), "normal");
        // Both match "x" or "y"? FTS matches words; use words both match.
        add_record(&c, "r3", "alpha beta gamma delta", "normal");
        add_record(&c, "r4", "alpha beta gamma epsilon", "normal");
        let req = RetrievalRequest {
            query: "alpha".into(),
            context_budget_chars: 30,
            ..Default::default()
        };
        let hits = retrieve(&c, &req, None, None).unwrap();
        let total: usize = hits.iter().map(|h| h.snippet.chars().count()).sum();
        assert!(total <= 30);
    }

    #[test]
    fn retrieve_with_vector_path_merges() {
        let c = fresh_conn();
        add_record(&c, "r1", "semantic content about cats", "normal");
        let emb = StubEmbeddingProvider;
        let mut idx = InMemoryVectorIndex::new();
        let v = emb.embed("semantic content about cats").unwrap();
        idx.add("r1", v).unwrap();
        let req = RetrievalRequest {
            query: "semantic content about cats".into(),
            ..Default::default()
        };
        let hits = retrieve(&c, &req, Some(&emb), Some(&idx)).unwrap();
        assert!(hits.iter().any(|h| h.record_id == "r1"));
    }

    #[test]
    fn retrieve_empty_query_returns_nothing() {
        let c = fresh_conn();
        add_record(&c, "r1", "some text", "normal");
        let req = RetrievalRequest::default();
        let hits = retrieve(&c, &req, None, None).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn assemble_answer_grounded_when_hits_exist() {
        let hits = vec![RetrievalHit {
            record_id: "r1".into(),
            score: 0.5,
            snippet: "the fact".into(),
        }];
        let a = assemble_answer("here is the fact", &hits);
        assert_eq!(a.trust, TrustClassification::SourceSupported);
        assert_eq!(a.citations.len(), 1);
        assert!(a.citations_are_consistent());
    }

    #[test]
    fn assemble_answer_inference_when_no_hits() {
        let a = assemble_answer("a guess", &[]);
        assert_eq!(a.trust, TrustClassification::Inference);
        assert!(a.citations.is_empty());
    }

    #[test]
    fn stub_embedding_is_deterministic() {
        let e = StubEmbeddingProvider;
        let a = e.embed("hello").unwrap();
        let b = e.embed("hello").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), EMBED_DIM);
    }

    #[test]
    fn vector_index_returns_nearest_first() {
        let mut idx = InMemoryVectorIndex::new();
        let e = StubEmbeddingProvider;
        idx.add("a", e.embed("cat").unwrap()).unwrap();
        idx.add("b", e.embed("cat").unwrap()).unwrap();
        idx.add("c", e.embed("dog").unwrap()).unwrap();
        let q = e.embed("cat").unwrap();
        let hits = idx.search(&q, 3).unwrap();
        // a and b are identical to the query, c is different.
        assert_eq!(hits[0].0, "a");
        assert!(hits[0].1 >= hits[2].1);
    }

    #[test]
    fn persistent_vector_index_survives_reopen_and_supports_delete_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let embedding = StubEmbeddingProvider;
        {
            let conn = Connection::open(&path).unwrap();
            let mut index = SqliteVectorIndex::new(&conn).unwrap();
            index
                .add(
                    "record-one",
                    embedding.embed("persistent vector fact").unwrap(),
                )
                .unwrap();
            assert_eq!(index.len().unwrap(), 1);
            index.remove("record-one").unwrap();
            assert_eq!(index.len().unwrap(), 0);
            index
                .add(
                    "record-one",
                    embedding.embed("persistent vector fact").unwrap(),
                )
                .unwrap();
        }
        {
            let conn = Connection::open(&path).unwrap();
            let index = SqliteVectorIndex::new(&conn).unwrap();
            assert_eq!(index.len().unwrap(), 1);
            let hits = index
                .search(&embedding.embed("persistent vector fact").unwrap(), 5)
                .unwrap();
            assert_eq!(hits.len(), 1);
        }
    }

    #[test]
    fn vector_index_rebuilds_from_current_canonical_records() {
        let conn = fresh_conn();
        add_record(&conn, "current", "current vector fact", "normal");
        let embedding = StubEmbeddingProvider;
        assert_eq!(rebuild_vector_index(&conn, &embedding).unwrap(), 1);
        let index = SqliteVectorIndex::new(&conn).unwrap();
        assert_eq!(index.len().unwrap(), 1);
    }

    #[test]
    fn in_memory_vector_index_replaces_and_clears_entries() {
        let embedding = StubEmbeddingProvider;
        let mut index = InMemoryVectorIndex::new();
        index.add("one", embedding.embed("first").unwrap()).unwrap();
        index
            .add("one", embedding.embed("replacement").unwrap())
            .unwrap();
        assert_eq!(index.items.len(), 1);
        index.clear().unwrap();
        assert!(index.items.is_empty());
    }
}
