//! `corpus` — versioned corpus import/export (Phase 6).
//!
//! Directive §20/§21. A corpus package is a versioned JSON document with a
//! manifest and an array of knowledge records. Export is read-only (never
//! modifies the vault). Import is idempotent: the same package imported twice
//! creates no duplicates (upsert by stable record id). Tombstones remove active
//! availability. Every record retains provenance.
//!
//! Package schemas live in `schemas/corpus-*.schema.json`. The CLI validator
//! (`sammy-cli corpus validate`) uses [`validate_package_bytes`].

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// The corpus package manifest (directive §20).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorpusManifest {
    pub manifest_version: u32,
    pub package_type: PackageType,
    pub vault_id: String,
    #[serde(default)]
    pub vault_name: String,
    pub created_at: String,
    pub app_version: String,
    pub record_schema_version: u32,
    pub record_count: u32,
    #[serde(default)]
    pub source_count: u32,
    /// SHA-256 of the records JSON array (hex).
    pub checksum: String,
    /// For incremental packages.
    #[serde(default)]
    pub previous_package_checksum: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    Full,
    Incremental,
}

/// A corpus record in the portable package format (subset of the DB row; only
/// the fields that are stable across export/import).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorpusRecord {
    pub record_id: String,
    #[serde(default)]
    pub vault_id: String,
    pub record_type: String,
    pub canonical_text: String,
    pub status: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub sensitivity: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub domain_tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub review_state: String,
    pub schema_version: u32,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub contradiction_set: Option<String>,
}

/// The full package: manifest + records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusPackage {
    pub manifest: CorpusManifest,
    pub records: Vec<CorpusRecord>,
}

pub const PACKAGE_FORMAT_VERSION: u32 = 1;
pub const RECORD_SCHEMA_VERSION: u32 = 1;

// -----------------------------------------------------------------------------
// Export
// -----------------------------------------------------------------------------

/// Export all knowledge records from the vault into a corpus package. Read-only
/// — never modifies the vault.
pub fn export_full(
    conn: &Connection,
    vault_id: &str,
    vault_name: &str,
) -> AppResult<CorpusPackage> {
    let records = export_records(conn)?;
    let records_json = serde_json::to_vec(&records)?;
    let checksum = hex::encode(Sha256::digest(&records_json));
    let manifest = CorpusManifest {
        manifest_version: PACKAGE_FORMAT_VERSION,
        package_type: PackageType::Full,
        vault_id: vault_id.to_string(),
        vault_name: vault_name.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        record_schema_version: RECORD_SCHEMA_VERSION,
        record_count: records.len() as u32,
        source_count: 0,
        checksum,
        previous_package_checksum: None,
    };
    Ok(CorpusPackage { manifest, records })
}

fn export_records(conn: &Connection) -> AppResult<Vec<CorpusRecord>> {
    let mut stmt = conn.prepare(
        "SELECT record_id, vault_id, record_type, canonical_text, status, source_ids,
                sensitivity, confidence, domain_tags, created_at, updated_at, review_state,
                supersedes, superseded_by, contradiction_set
         FROM knowledge_records_full ORDER BY created_at",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CorpusRecord {
            record_id: r.get::<_, String>(0)?,
            vault_id: r.get::<_, String>(1)?,
            record_type: r.get::<_, String>(2)?,
            canonical_text: r.get::<_, String>(3)?,
            status: r.get::<_, String>(4)?,
            source_ids: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or_default(),
            sensitivity: r.get::<_, String>(6)?,
            confidence: r.get::<_, f64>(7)?,
            domain_tags: serde_json::from_str(&r.get::<_, String>(8)?)
                .unwrap_or_default(),
            created_at: r.get::<_, String>(9)?,
            updated_at: r.get::<_, String>(10)?,
            review_state: r.get::<_, String>(11)?,
            supersedes: r.get::<_, Option<String>>(12)?,
            superseded_by: r.get::<_, Option<String>>(13)?,
            contradiction_set: r.get::<_, Option<String>>(14)?,
            schema_version: RECORD_SCHEMA_VERSION,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// -----------------------------------------------------------------------------
// Validation
// -----------------------------------------------------------------------------

/// Validate a package's structure: manifest fields present, checksum matches
/// the records, record ids are unique, no malformed required fields.
pub fn validate_package(pkg: &CorpusPackage) -> AppResult<()> {
    if pkg.manifest.manifest_version != PACKAGE_FORMAT_VERSION {
        return Err(AppError::Config(format!(
            "unsupported manifest version {}",
            pkg.manifest.manifest_version
        )));
    }
    if pkg.manifest.record_schema_version != RECORD_SCHEMA_VERSION {
        return Err(AppError::Config(format!(
            "unsupported record schema version {}",
            pkg.manifest.record_schema_version
        )));
    }
    // Checksum: recompute over the records array and compare.
    let records_json = serde_json::to_vec(&pkg.records)?;
    let actual = hex::encode(Sha256::digest(&records_json));
    if actual != pkg.manifest.checksum {
        return Err(AppError::Config(
            "package checksum mismatch — records may be corrupted".into(),
        ));
    }
    if pkg.manifest.record_count as usize != pkg.records.len() {
        return Err(AppError::Config(
            "manifest record_count does not match records length".into(),
        ));
    }
    // Record ids must be unique.
    let mut seen = std::collections::HashSet::new();
    for r in &pkg.records {
        if r.record_id.is_empty() {
            return Err(AppError::Config("record with empty id".into()));
        }
        if !seen.insert(&r.record_id) {
            return Err(AppError::Config(format!(
                "duplicate record id in package: {}",
                r.record_id
            )));
        }
    }
    Ok(())
}

/// Validate a package from raw JSON bytes (used by the CLI validator).
pub fn validate_package_bytes(json: &[u8]) -> AppResult<()> {
    let pkg: CorpusPackage = serde_json::from_slice(json)
        .map_err(|e| AppError::Config(format!("invalid package JSON: {e}")))?;
    validate_package(&pkg)
}

/// Serialize a package to pretty JSON bytes (for writing to a file).
pub fn package_to_json(pkg: &CorpusPackage) -> AppResult<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(pkg)?)
}

/// Parse a package from JSON bytes.
pub fn package_from_json(json: &[u8]) -> AppResult<CorpusPackage> {
    let pkg: CorpusPackage = serde_json::from_slice(json)
        .map_err(|e| AppError::Config(format!("invalid package: {e}")))?;
    Ok(pkg)
}

// -----------------------------------------------------------------------------
// Import (idempotent upsert by record id)
// -----------------------------------------------------------------------------

/// Outcome of importing a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOutcome {
    pub imported: u32,
    pub updated: u32,
    pub tombstoned: u32,
    pub skipped: u32,
}

/// Import a validated package into the vault. Idempotent: re-importing the same
/// package updates existing records (upsert by record_id) rather than creating
/// duplicates. Tombstoned records in the package remove active availability.
/// The `vault_id` is the *destination* vault (records are re-tagged to it so a
/// package from one vault imports cleanly into another).
pub fn import(
    conn: &Connection,
    vault_id: &str,
    pkg: &CorpusPackage,
) -> AppResult<ImportOutcome> {
    // Always validate before importing.
    validate_package(pkg)?;

    let mut imported = 0u32;
    let mut updated = 0u32;
    let mut tombstoned = 0u32;
    let mut skipped = 0u32;

    for r in &pkg.records {
        let exists = record_exists(conn, &r.record_id)?;
        // Re-tag to the destination vault.
        let dest_vault = vault_id.to_string();
        if r.status == "tombstoned" {
            // Tombstone: remove active availability (directive §20).
            if exists {
                apply_tombstone(conn, &r.record_id)?;
                tombstoned += 1;
            } else {
                // Insert as already-tombstoned so re-import doesn't resurrect.
                insert_record(conn, &dest_vault, r)?;
                tombstoned += 1;
            }
            continue;
        }
        if exists {
            // Update existing (upsert). Update text/status if the package's
            // updated_at is newer; otherwise skip (idempotent no-op).
            if should_update(conn, r)? {
                update_record(conn, &dest_vault, r)?;
                updated += 1;
            } else {
                skipped += 1;
            }
        } else {
            insert_record(conn, &dest_vault, r)?;
            imported += 1;
        }
    }

    Ok(ImportOutcome {
        imported,
        updated,
        tombstoned,
        skipped,
    })
}

fn record_exists(conn: &Connection, id: &str) -> AppResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM knowledge_records_full WHERE record_id=?1",
        params![id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

fn should_update(conn: &Connection, r: &CorpusRecord) -> AppResult<bool> {
    let existing_updated: Option<String> = conn
        .query_row(
            "SELECT updated_at FROM knowledge_records_full WHERE record_id=?1",
            params![r.record_id],
            |row| row.get(0),
        )
        .ok();
    Ok(existing_updated.as_deref() < Some(r.updated_at.as_str()))
}

fn insert_record(conn: &Connection, vault_id: &str, r: &CorpusRecord) -> AppResult<()> {
    conn.execute(
        "INSERT INTO knowledge_records_full(
            record_id, vault_id, record_type, canonical_text, status, source_ids,
            sensitivity, confidence, domain_tags, created_at, updated_at, review_state,
            supersedes, superseded_by, contradiction_set, schema_version)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            r.record_id,
            vault_id,
            r.record_type,
            r.canonical_text,
            r.status,
            serde_json::to_string(&r.source_ids)?,
            r.sensitivity,
            r.confidence,
            serde_json::to_string(&r.domain_tags)?,
            r.created_at,
            r.updated_at,
            r.review_state,
            r.supersedes,
            r.superseded_by,
            r.contradiction_set,
            r.schema_version,
        ],
    )?;
    if r.status == "approved" || r.status == "disputed" {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO knowledge_fts(record_id, canonical_text) VALUES (?1,?2)",
            params![r.record_id, r.canonical_text],
        );
    }
    Ok(())
}

fn update_record(conn: &Connection, vault_id: &str, r: &CorpusRecord) -> AppResult<()> {
    conn.execute(
        "UPDATE knowledge_records_full SET
            vault_id=?2, record_type=?3, canonical_text=?4, status=?5, source_ids=?6,
            sensitivity=?7, confidence=?8, domain_tags=?9, updated_at=?11, review_state=?12,
            supersedes=?13, superseded_by=?14, contradiction_set=?15
         WHERE record_id=?1",
        params![
            r.record_id,
            vault_id,
            r.record_type,
            r.canonical_text,
            r.status,
            serde_json::to_string(&r.source_ids)?,
            r.sensitivity,
            r.confidence,
            serde_json::to_string(&r.domain_tags)?,
            r.created_at,
            r.updated_at,
            r.review_state,
            r.supersedes,
            r.superseded_by,
            r.contradiction_set,
        ],
    )?;
    // Keep FTS in sync: remove then re-add if current.
    let _ = conn.execute(
        "DELETE FROM knowledge_fts WHERE record_id=?1",
        params![r.record_id],
    );
    if r.status == "approved" || r.status == "disputed" {
        let _ = conn.execute(
            "INSERT INTO knowledge_fts(record_id, canonical_text) VALUES (?1,?2)",
            params![r.record_id, r.canonical_text],
        );
    }
    Ok(())
}

fn apply_tombstone(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE knowledge_records_full SET status='tombstoned', review_state='tombstoned',
            updated_at=?2 WHERE record_id=?1",
        params![id, chrono::Utc::now().to_rfc3339()],
    )?;
    let _ = conn.execute("DELETE FROM knowledge_fts WHERE record_id=?1", params![id]);
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

    fn seed(conn: &Connection, id: &str, text: &str) {
        conn.execute(
            "INSERT INTO knowledge_records_full(record_id, vault_id, record_type, canonical_text,
                status, created_at, updated_at, review_state)
             VALUES (?1,'src','fact',?2,'approved','2020-01-01T00:00:00Z','2020-01-01T00:00:00Z','approved')",
            params![id, text],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_fts(record_id, canonical_text) VALUES (?1,?2)",
            params![id, text],
        )
        .unwrap();
    }

    #[test]
    fn export_then_import_into_clean_vault_is_idempotent() {
        let src = fresh_conn();
        seed(&src, "r1", "first fact");
        seed(&src, "r2", "second fact");
        let pkg = export_full(&src, "vsrc", "Source").unwrap();

        let dest = fresh_conn();
        let o1 = import(&dest, "vdest", &pkg).unwrap();
        assert_eq!(o1.imported, 2);
        // Re-import the same package: should be idempotent (no duplicates).
        let o2 = import(&dest, "vdest", &pkg).unwrap();
        assert_eq!(o2.imported, 0);
        assert_eq!(o2.skipped, 2); // same updated_at, no-op
                                   // Count rows: still 2.
        let count: i64 = dest
            .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn import_applies_updates_when_newer() {
        let src = fresh_conn();
        seed(&src, "r1", "first fact");
        let pkg = export_full(&src, "vsrc", "S").unwrap();

        let dest = fresh_conn();
        import(&dest, "vdest", &pkg).unwrap();

        // Build an updated package with newer updated_at and changed text.
        let mut pkg2 = pkg.clone();
        pkg2.records[0].canonical_text = "first fact (revised)".into();
        pkg2.records[0].updated_at = "2030-01-01T00:00:00Z".into();
        pkg2.records[0].status = "approved".into();
        // Recompute checksum.
        let rj = serde_json::to_vec(&pkg2.records).unwrap();
        pkg2.manifest.checksum = hex::encode(Sha256::digest(&rj));

        let o = import(&dest, "vdest", &pkg2).unwrap();
        assert_eq!(o.updated, 1);
        let text: String = dest
            .query_row(
                "SELECT canonical_text FROM knowledge_records_full WHERE record_id='r1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "first fact (revised)");
    }

    #[test]
    fn import_tombstone_removes_active_availability() {
        let src = fresh_conn();
        seed(&src, "r1", "to be tombstoned");
        let pkg = export_full(&src, "vsrc", "S").unwrap();

        let dest = fresh_conn();
        import(&dest, "vdest", &pkg).unwrap();

        // Build a package that tombstones r1.
        let mut pkg2 = pkg.clone();
        pkg2.records[0].status = "tombstoned".into();
        pkg2.records[0].review_state = "tombstoned".into();
        let rj = serde_json::to_vec(&pkg2.records).unwrap();
        pkg2.manifest.checksum = hex::encode(Sha256::digest(&rj));

        let o = import(&dest, "vdest", &pkg2).unwrap();
        assert_eq!(o.tombstoned, 1);
        let status: String = dest
            .query_row(
                "SELECT status FROM knowledge_records_full WHERE record_id='r1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "tombstoned");
    }

    #[test]
    fn validate_rejects_bad_checksum() {
        let src = fresh_conn();
        seed(&src, "r1", "fact");
        let mut pkg = export_full(&src, "v", "S").unwrap();
        pkg.manifest.checksum = "deadbeef".into();
        assert!(validate_package(&pkg).is_err());
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let mut pkg = CorpusPackage {
            manifest: CorpusManifest {
                manifest_version: 1,
                package_type: PackageType::Full,
                vault_id: "v".into(),
                vault_name: "S".into(),
                created_at: "t".into(),
                app_version: "0.1".into(),
                record_schema_version: 1,
                record_count: 2,
                source_count: 0,
                checksum: "x".into(),
                previous_package_checksum: None,
            },
            records: vec![
                CorpusRecord {
                    record_id: "dup".into(),
                    vault_id: "v".into(),
                    record_type: "fact".into(),
                    canonical_text: "a".into(),
                    status: "approved".into(),
                    source_ids: vec![],
                    sensitivity: "normal".into(),
                    confidence: 1.0,
                    domain_tags: vec![],
                    created_at: "t".into(),
                    updated_at: "t".into(),
                    review_state: "approved".into(),
                    schema_version: 1,
                    supersedes: None,
                    superseded_by: None,
                    contradiction_set: None,
                },
                CorpusRecord {
                    record_id: "dup".into(),
                    vault_id: "v".into(),
                    record_type: "fact".into(),
                    canonical_text: "b".into(),
                    status: "approved".into(),
                    source_ids: vec![],
                    sensitivity: "normal".into(),
                    confidence: 1.0,
                    domain_tags: vec![],
                    created_at: "t".into(),
                    updated_at: "t".into(),
                    review_state: "approved".into(),
                    schema_version: 1,
                    supersedes: None,
                    superseded_by: None,
                    contradiction_set: None,
                },
            ],
        };
        // Fix checksum so the only failure is duplicate ids.
        let rj = serde_json::to_vec(&pkg.records).unwrap();
        pkg.manifest.checksum = hex::encode(Sha256::digest(&rj));
        assert!(validate_package(&pkg).is_err());
    }

    #[test]
    fn package_round_trips_through_json() {
        let src = fresh_conn();
        seed(&src, "r1", "fact");
        let pkg = export_full(&src, "v", "S").unwrap();
        let json = package_to_json(&pkg).unwrap();
        let back = package_from_json(&json).unwrap();
        assert_eq!(back.manifest, pkg.manifest);
        assert_eq!(back.records, pkg.records);
        validate_package_bytes(&json).unwrap();
    }
}
