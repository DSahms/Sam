//! Versioned database migrations with rollback protection (Phase 1).
//!
//! Directive §37 forbids "database migration without rollback protection."
//! Our design:
//! - Migrations are forward-only and identified by a monotonically increasing
//!   version number.
//! - Each migration runs inside a SQL transaction. If it fails, the
//!   transaction rolls back, leaving the database at its previous version.
//! - The `schema_version` row is updated only after a migration's transaction
//!   commits, so a crash mid-migration never records an un-applied version.
//! - On open, [`run_pending`] applies all migrations newer than the recorded
//!   version. There is no automatic downgrade; a failed migration is a hard
//!   error the caller must report (and the DB remains usable at its last good
//!   version).
//!
//! The baseline schema (version 1: `audit_events`, `schema_version`) is applied
//! at vault creation by the vault module; this module owns everything *after*
//! version 1 and is also idempotent if version 1 is not yet recorded.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// A single forward migration. `version` is the schema version this migration
/// produces. `sql` MUST be transaction-safe (no `ATTACH`, no explicit
/// `COMMIT`); it is wrapped in a transaction by the runner.
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// The ordered set of migrations beyond the baseline (version 1).
/// Add new entries here as subsystems evolve. Versions MUST be contiguous and
/// increasing.
pub const MIGRATIONS: &[Migration] = &[
    // Version 2: knowledge records core (Phase 3 will populate columns).
    Migration {
        version: 2,
        description: "knowledge_records table",
        sql: "CREATE TABLE IF NOT EXISTS knowledge_records (
                record_id      TEXT PRIMARY KEY,
                record_type    TEXT NOT NULL,
                canonical_text TEXT NOT NULL,
                status         TEXT NOT NULL,
                sensitivity    TEXT NOT NULL DEFAULT 'normal',
                created_at     TEXT NOT NULL,
                updated_at     TEXT NOT NULL,
                schema_version INTEGER NOT NULL DEFAULT 1
              );
              CREATE INDEX IF NOT EXISTS knowledge_records_status
                  ON knowledge_records(status);",
    },
    // Version 3: conversations and messages (Phase 2 will use these).
    Migration {
        version: 3,
        description: "conversations and messages",
        sql: "CREATE TABLE IF NOT EXISTS conversations (
                conversation_id TEXT PRIMARY KEY,
                title           TEXT,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
              );
              CREATE TABLE IF NOT EXISTS messages (
                message_id      TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(conversation_id),
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                seq             INTEGER NOT NULL
              );
              CREATE INDEX IF NOT EXISTS messages_conversation
                  ON messages(conversation_id, seq);",
    },
    // Version 4: sources (Phase 4 will populate columns).
    Migration {
        version: 4,
        description: "sources table",
        sql: "CREATE TABLE IF NOT EXISTS sources (
                source_id   TEXT PRIMARY KEY,
                kind        TEXT NOT NULL,
                name        TEXT NOT NULL,
                checksum    TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'active'
              );",
    },
];

/// The highest migration version defined.
pub fn latest_version() -> u32 {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(1)
}

/// Read the currently-applied schema version from the database. Returns 0 if
/// the table does not exist yet (a brand-new connection before baseline).
pub fn current_version(conn: &Connection) -> AppResult<u32> {
    let exists: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Ok(0);
    }
    let v: Option<i64> =
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
    Ok(v.unwrap_or(0) as u32)
}

/// Apply all pending migrations in order. Each migration runs in its own
/// transaction; on failure the transaction rolls back and an error is returned
/// without advancing the recorded version.
pub fn run_pending(conn: &mut Connection) -> AppResult<u32> {
    let mut current = current_version(conn)?;
    for m in MIGRATIONS.iter() {
        if m.version <= current {
            continue;
        }
        apply_one(conn, m)?;
        current = m.version;
    }
    Ok(current)
}

/// Apply a single migration transactionally, then record its version.
fn apply_one(conn: &mut Connection, m: &Migration) -> AppResult<()> {
    // Ensure schema_version exists (idempotent with vault baseline).
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
        [],
    )?;
    // Already applied? Skip.
    let already: i64 = conn.query_row(
        "SELECT count(*) FROM schema_version WHERE version = ?1",
        rusqlite::params![m.version as i64],
        |r| r.get(0),
    )?;
    if already > 0 {
        return Ok(());
    }

    let tx = conn.transaction()?;
    tx.execute_batch(m.sql).map_err(|e| {
        // Directive §37: never silently leave a half-applied migration.
        // Rolling back (via Drop) and returning an error satisfies this.
        log::error!(
            "migration v{} ({}) failed; rolled back: {}",
            m.version,
            m.description,
            e
        );
        AppError::Database
    })?;
    tx.execute(
        "INSERT INTO schema_version(version, applied_at) VALUES (?, ?)",
        rusqlite::params![m.version as i64, now_iso()],
    )?;
    tx.commit()?;
    log::info!("migration v{} ({}) applied", m.version, m.description);
    Ok(())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn current_version_is_zero_on_empty_db() {
        let c = fresh_conn();
        assert_eq!(current_version(&c).unwrap(), 0);
    }

    #[test]
    fn run_pending_applies_all_migrations_in_order() {
        let mut c = fresh_conn();
        let final_v = run_pending(&mut c).unwrap();
        assert_eq!(final_v, latest_version());
        assert_eq!(current_version(&c).unwrap(), latest_version());
        // Tables created.
        let count: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN \
                 ('knowledge_records','conversations','messages','sources','schema_version')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn run_pending_is_idempotent() {
        let mut c = fresh_conn();
        run_pending(&mut c).unwrap();
        let v1 = current_version(&c).unwrap();
        run_pending(&mut c).unwrap();
        let v2 = current_version(&c).unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn failed_migration_rolls_back_and_keeps_prior_version() {
        // Inject a bad migration after a good one by simulating: apply v2/3/4,
        // then attempt a migration with invalid SQL and assert it fails AND the
        // version did not advance past latest_version().
        let mut c = fresh_conn();
        run_pending(&mut c).unwrap();
        let good_version = current_version(&c).unwrap();

        let bad = Migration {
            version: good_version + 1,
            description: "intentionally broken",
            sql: "THIS IS NOT SQL;",
        };
        let err = apply_one(&mut c, &bad);
        assert!(err.is_err(), "broken migration must fail");
        // Version unchanged.
        assert_eq!(current_version(&c).unwrap(), good_version);
    }

    #[test]
    fn each_migration_version_is_contiguous_and_increasing() {
        let mut prev = 1u32;
        for m in MIGRATIONS {
            assert_eq!(
                m.version,
                prev + 1,
                "migration versions must be contiguous; saw {} after {}",
                m.version,
                prev
            );
            prev = m.version;
        }
    }

    #[test]
    fn migration_tables_are_writable() {
        let mut c = fresh_conn();
        run_pending(&mut c).unwrap();
        c.execute(
            "INSERT INTO knowledge_records(record_id, record_type, canonical_text, status, created_at, updated_at) \
             VALUES ('r1','fact','sky is blue','approved','t','t')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO conversations(conversation_id, created_at, updated_at) VALUES ('c1','t','t')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO messages(message_id, conversation_id, role, content, created_at, seq) \
             VALUES ('m1','c1','user','hi','t',1)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO sources(source_id, kind, name, checksum, imported_at) \
             VALUES ('s1','file','a.txt','abc','t')",
            [],
        )
        .unwrap();
        assert_eq!(count(&c, "knowledge_records"), 1);
        assert_eq!(count(&c, "messages"), 1);
        assert_eq!(count(&c, "sources"), 1);
    }

    fn count(c: &Connection, table: &str) -> i64 {
        c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}
