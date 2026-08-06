//! `audit` — append-only audit history API (Phase 1 foundation).
//!
//! Records security-relevant events per vault in the `audit_events` table.
//! Per directive §13 and §19, audit events must never contain deleted private
//! content or full private prompts; callers pass only non-sensitive detail.
//!
//! The table is append-only by convention: this module exposes only
//! [`record`] and [`list`]; there is no `delete` or `update` for audit rows.
//! Row integrity is enforced by making `seq` an autoincrement primary key and
//! never exposing a mutation path.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::ids::AuditEventId;

/// The broad category of an audit event. Keeps the `category` column stable
/// and queryable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Vault,
    Provider,
    Source,
    Knowledge,
    Memory,
    Permission,
    Backup,
    Export,
    Security,
}

impl AuditCategory {
    fn as_str(self) -> &'static str {
        match self {
            AuditCategory::Vault => "vault",
            AuditCategory::Provider => "provider",
            AuditCategory::Source => "source",
            AuditCategory::Knowledge => "knowledge",
            AuditCategory::Memory => "memory",
            AuditCategory::Permission => "permission",
            AuditCategory::Backup => "backup",
            AuditCategory::Export => "export",
            AuditCategory::Security => "security",
        }
    }
}

/// One audit row. `detail_json` carries non-sensitive structured detail chosen
/// by the caller (e.g. `{"provider":"mock","model":"m1"}` for a provider call).
/// It MUST NOT contain secrets, full prompts, or deleted private content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: i64,
    pub event_id: String,
    pub occurred_at: String,
    pub actor: Option<String>,
    pub category: String,
    pub action: String,
    pub detail_json: serde_json::Value,
}

/// Record an audit event. `action` is a short verb (e.g. "vault_unlock",
/// "provider_call", "source_import"). `detail` is serialized to JSON and must
/// be non-sensitive.
pub fn record(
    conn: &Connection,
    category: AuditCategory,
    action: &str,
    actor: Option<&str>,
    detail: &serde_json::Value,
) -> AppResult<AuditEventId> {
    let event_id = AuditEventId::new();
    let occurred_at = chrono::Utc::now().to_rfc3339();
    let detail_str = serde_json::to_string(detail)?;
    conn.execute(
        "INSERT INTO audit_events(event_id, occurred_at, actor, category, action, detail_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id.to_string(),
            occurred_at,
            actor,
            category.as_str(),
            action,
            detail_str,
        ],
    )?;
    Ok(event_id)
}

/// List audit events, most-recent-first, up to `limit`.
pub fn list(conn: &Connection, limit: i64) -> AppResult<Vec<AuditEvent>> {
    let mut stmt = conn.prepare(
        "SELECT seq, event_id, occurred_at, actor, category, action, detail_json
         FROM audit_events
         ORDER BY seq DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        let detail_str: String = row.get(6)?;
        let detail: serde_json::Value =
            serde_json::from_str(&detail_str).unwrap_or(serde_json::Value::Null);
        Ok(AuditEvent {
            seq: row.get(0)?,
            event_id: row.get(1)?,
            occurred_at: row.get(2)?,
            actor: row.get(3)?,
            category: row.get(4)?,
            action: row.get(5)?,
            detail_json: detail,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Count audit events by category. Useful for the Privacy & Audit view.
pub fn count_by_category(conn: &Connection) -> AppResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT category, count(*) FROM audit_events GROUP BY category ORDER BY category",
    )?;
    let rows =
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Assert (in tests) that a given action was recorded at least once.
#[cfg(test)]
pub(crate) fn assert_action_recorded(conn: &Connection, action: &str) -> bool {
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM audit_events WHERE action = ?1",
            params![action],
            |r| r.get(0),
        )
        .unwrap_or(0);
    n > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute(
            "CREATE TABLE audit_events (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL UNIQUE,
                occurred_at TEXT NOT NULL,
                actor TEXT,
                category TEXT NOT NULL,
                action TEXT NOT NULL,
                detail_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        )
        .unwrap();
        c
    }

    #[test]
    fn record_then_list() {
        let c = fresh_conn();
        record(
            &c,
            AuditCategory::Vault,
            "vault_unlock",
            Some("owner"),
            &serde_json::json!({"method": "passphrase"}),
        )
        .unwrap();
        record(
            &c,
            AuditCategory::Provider,
            "provider_call",
            None,
            &serde_json::json!({"provider": "mock", "model": "m1"}),
        )
        .unwrap();

        let events = list(&c, 10).unwrap();
        assert_eq!(events.len(), 2);
        // Most-recent-first: provider_call recorded last.
        assert_eq!(events[0].action, "provider_call");
        assert_eq!(events[0].category, "provider");
        assert_eq!(events[1].action, "vault_unlock");
        assert_eq!(events[1].detail_json["method"], "passphrase");
    }

    #[test]
    fn count_by_category_groups_correctly() {
        let c = fresh_conn();
        for _ in 0..3 {
            record(
                &c,
                AuditCategory::Vault,
                "vault_lock",
                None,
                &serde_json::json!({}),
            )
            .unwrap();
        }
        record(
            &c,
            AuditCategory::Security,
            "denied",
            None,
            &serde_json::json!({}),
        )
        .unwrap();
        let counts = count_by_category(&c).unwrap();
        let map: std::collections::HashMap<String, i64> = counts.into_iter().collect();
        assert_eq!(map["vault"], 3);
        assert_eq!(map["security"], 1);
    }

    #[test]
    fn seq_is_monotonic() {
        let c = fresh_conn();
        let _ =
            record(&c, AuditCategory::Vault, "a", None, &serde_json::json!({})).unwrap();
        let _ =
            record(&c, AuditCategory::Vault, "b", None, &serde_json::json!({})).unwrap();
        let events = list(&c, 10).unwrap();
        // most-recent-first: seq descending
        assert!(events[0].seq > events[1].seq);
    }

    #[test]
    fn assert_action_recorded_helper_works() {
        let c = fresh_conn();
        record(
            &c,
            AuditCategory::Backup,
            "backup_create",
            None,
            &serde_json::json!({}),
        )
        .unwrap();
        assert!(assert_action_recorded(&c, "backup_create"));
        assert!(!assert_action_recorded(&c, "never_happened"));
    }
}
