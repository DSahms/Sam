//! `permissions` — permission architecture (Phase 8).
//!
//! Directive §28/§29. Every tool action declares what it does, what data it
//! accesses, what data leaves the device, its risk level, and its
//! reversibility. Permission modes range from always-deny to allow-once /
//! session / narrow-scope. No conversational request becomes permanent
//! authorization. Revocation is immediate.
//!
//! The first release executes NO external actions; the permission architecture
//! gates future tools. Mock tools exist for permission testing.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    ReadOnly,
    DraftOnly,
    SendsData,
    MutatesExternal,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::ReadOnly => "read_only",
            RiskLevel::DraftOnly => "draft_only",
            RiskLevel::SendsData => "sends_data",
            RiskLevel::MutatesExternal => "mutates_external",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    Reversible,
    Irreversible,
}

/// A tool's static declaration (directive §29).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDeclaration {
    pub tool_id: String,
    pub operation: String,
    pub data_accessed: String,
    pub data_leaving_device: String,
    pub destination: String,
    pub risk: RiskLevel,
    pub reversibility: Reversibility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    AlwaysDeny,
    AskEveryTime,
    AllowOnce,
    AllowSession,
    AllowScoped,
}

/// A request to invoke a tool, with an action preview (directive §29).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool_id: String,
    pub operation: String,
    pub data_accessed: String,
    pub data_leaving_device: String,
    pub destination: String,
    pub risk: RiskLevel,
    pub reversibility: Reversibility,
    pub requested_mode: PermissionMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub grant_id: String,
    pub tool_id: String,
    pub mode: String,
    pub scope: String,
    pub granted_at: String,
    pub expires_at: Option<String>,
}

pub fn ensure_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS permission_grants (
            grant_id   TEXT PRIMARY KEY,
            tool_id    TEXT NOT NULL,
            mode       TEXT NOT NULL,
            scope      TEXT NOT NULL DEFAULT '',
            granted_at TEXT NOT NULL,
            expires_at TEXT,
            revoked    INTEGER NOT NULL DEFAULT 0
        );
         CREATE INDEX IF NOT EXISTS pg_tool ON permission_grants(tool_id);",
    )?;
    Ok(())
}

/// Check whether a tool invocation is permitted given existing grants.
pub fn check_permission(
    conn: &Connection,
    tool_id: &str,
) -> AppResult<Option<PermissionGrant>> {
    ensure_schema(conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT grant_id, tool_id, mode, scope, granted_at, expires_at
         FROM permission_grants WHERE tool_id=?1 AND revoked=0
         AND (expires_at IS NULL OR expires_at > ?2)
         ORDER BY granted_at DESC LIMIT 1",
    )?;
    let grant = stmt
        .query_row(params![tool_id, now], |r| {
            Ok(PermissionGrant {
                grant_id: r.get::<_, String>(0)?,
                tool_id: r.get::<_, String>(1)?,
                mode: r.get::<_, String>(2)?,
                scope: r.get::<_, String>(3)?,
                granted_at: r.get::<_, String>(4)?,
                expires_at: r.get::<_, Option<String>>(5)?,
            })
        })
        .ok();
    Ok(grant)
}

/// Record a permission grant after the owner approves.
pub fn grant(
    conn: &Connection,
    tool_id: &str,
    mode: PermissionMode,
    scope: &str,
    session_expires_at: Option<&str>,
) -> AppResult<String> {
    ensure_schema(conn)?;
    let grant_id = crate::ids::AuditEventId::new().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO permission_grants(grant_id, tool_id, mode, scope, granted_at, expires_at, revoked)
         VALUES (?1,?2,?3,?4,?5,?6,0)",
        params![
            grant_id,
            tool_id,
            mode_as_str(mode),
            scope,
            now,
            session_expires_at,
        ],
    )?;
    Ok(grant_id)
}

/// Revoke all grants for a tool. Immediate (directive §29).
pub fn revoke(conn: &Connection, tool_id: &str) -> AppResult<()> {
    ensure_schema(conn)?;
    conn.execute(
        "UPDATE permission_grants SET revoked=1 WHERE tool_id=?1",
        params![tool_id],
    )?;
    Ok(())
}

/// Revoke a single grant by id.
pub fn revoke_grant(conn: &Connection, grant_id: &str) -> AppResult<()> {
    ensure_schema(conn)?;
    conn.execute(
        "UPDATE permission_grants SET revoked=1 WHERE grant_id=?1",
        params![grant_id],
    )?;
    Ok(())
}

/// List all active grants.
pub fn list_active(conn: &Connection) -> AppResult<Vec<PermissionGrant>> {
    ensure_schema(conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT grant_id, tool_id, mode, scope, granted_at, expires_at
         FROM permission_grants WHERE revoked=0
         AND (expires_at IS NULL OR expires_at > ?1)
         ORDER BY granted_at DESC",
    )?;
    let rows = stmt.query_map(params![now], |r| {
        Ok(PermissionGrant {
            grant_id: r.get::<_, String>(0)?,
            tool_id: r.get::<_, String>(1)?,
            mode: r.get::<_, String>(2)?,
            scope: r.get::<_, String>(3)?,
            granted_at: r.get::<_, String>(4)?,
            expires_at: r.get::<_, Option<String>>(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn mode_as_str(m: PermissionMode) -> &'static str {
    match m {
        PermissionMode::AlwaysDeny => "always_deny",
        PermissionMode::AskEveryTime => "ask_every_time",
        PermissionMode::AllowOnce => "allow_once",
        PermissionMode::AllowSession => "allow_session",
        PermissionMode::AllowScoped => "allow_scoped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        ensure_schema(&c).unwrap();
        c
    }

    #[test]
    fn no_grant_returns_none() {
        let c = fresh();
        assert!(check_permission(&c, "source_search").unwrap().is_none());
    }

    #[test]
    fn grant_then_check_returns_active() {
        let c = fresh();
        grant(&c, "source_search", PermissionMode::AllowSession, "", None).unwrap();
        let g = check_permission(&c, "source_search").unwrap();
        assert!(g.is_some());
        assert_eq!(g.unwrap().mode, "allow_session");
    }

    #[test]
    fn revoke_is_immediate() {
        let c = fresh();
        grant(&c, "draft_gen", PermissionMode::AllowScoped, "conv-1", None).unwrap();
        assert!(check_permission(&c, "draft_gen").unwrap().is_some());
        revoke(&c, "draft_gen").unwrap();
        assert!(check_permission(&c, "draft_gen").unwrap().is_none());
    }

    #[test]
    fn expired_grant_not_active() {
        let c = fresh();
        grant(
            &c,
            "mock_tool",
            PermissionMode::AllowSession,
            "",
            Some("2000-01-01T00:00:00Z"),
        )
        .unwrap();
        assert!(check_permission(&c, "mock_tool").unwrap().is_none());
    }

    #[test]
    fn list_active_excludes_revoked_and_expired() {
        let c = fresh();
        grant(&c, "a", PermissionMode::AllowSession, "", None).unwrap();
        grant(
            &c,
            "b",
            PermissionMode::AllowSession,
            "",
            Some("2000-01-01T00:00:00Z"),
        )
        .unwrap();
        revoke(&c, "a").unwrap();
        assert!(list_active(&c).unwrap().is_empty());
    }

    #[test]
    fn revoke_grant_by_id() {
        let c = fresh();
        let gid = grant(&c, "x", PermissionMode::AllowOnce, "", None).unwrap();
        assert!(check_permission(&c, "x").unwrap().is_some());
        revoke_grant(&c, &gid).unwrap();
        assert!(check_permission(&c, "x").unwrap().is_none());
    }
}
