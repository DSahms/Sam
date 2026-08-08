//! `conversation` — conversation and message model (Phase 2).
//!
//! Conversations and messages live in the vault's SQLCipher DB (the
//! `conversations` and `messages` tables created by migration v3). Conversation
//! summaries may support navigation but never silently redefine identity or
//! knowledge (directive §26).
//!
//! Message roles follow the standard `system / user / assistant / tool`
//! convention so any provider adapter can consume them without translation.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::ids::{ConversationId, MessageId};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Role::System),
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            "tool" => Some(Role::Tool),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub conversation_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub seq: i64,
}

/// Provider/model metadata recorded for every assistant message (directive
/// §13: each response records provider and model metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMeta {
    pub provider: String,
    pub model: String,
    pub routing_mode: String,
    /// Whether the turn's content crossed to a cloud provider.
    pub crossed_to_cloud: bool,
}

/// Create a new conversation. Returns its id.
pub fn create(conn: &Connection, title: Option<&str>) -> AppResult<ConversationId> {
    let id = ConversationId::new();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO conversations(conversation_id, title, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)",
        params![id.to_string(), title, now],
    )?;
    Ok(id)
}

/// Append a message to a conversation. Returns the new message id. Bumps the
/// conversation's updated_at.
pub fn append_message(
    conn: &Connection,
    conversation_id: ConversationId,
    role: Role,
    content: &str,
) -> AppResult<MessageId> {
    let id = MessageId::new();
    let now = chrono::Utc::now().to_rfc3339();
    // next seq
    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE conversation_id = ?1",
            params![conversation_id.to_string()],
            |r| r.get(0),
        )
        .unwrap_or(1);
    conn.execute(
        "INSERT INTO messages(message_id, conversation_id, role, content, created_at, seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.to_string(),
            conversation_id.to_string(),
            role.as_str(),
            content,
            now,
            next_seq
        ],
    )?;
    conn.execute(
        "UPDATE conversations SET updated_at = ?1 WHERE conversation_id = ?2",
        params![now, conversation_id.to_string()],
    )?;
    Ok(id)
}

/// List conversations, most-recently-updated first.
pub fn list(conn: &Connection) -> AppResult<Vec<Conversation>> {
    let mut stmt = conn.prepare(
        "SELECT conversation_id, title, created_at, updated_at
         FROM conversations ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Conversation {
            conversation_id: r.get::<_, String>(0)?,
            title: r.get::<_, Option<String>>(1)?,
            created_at: r.get::<_, String>(2)?,
            updated_at: r.get::<_, String>(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Load all messages of a conversation in seq order.
pub fn messages(
    conn: &Connection,
    conversation_id: ConversationId,
) -> AppResult<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT message_id, conversation_id, role, content, created_at, seq
         FROM messages WHERE conversation_id = ?1 ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id.to_string()], |r| {
        Ok(Message {
            message_id: r.get::<_, String>(0)?,
            conversation_id: r.get::<_, String>(1)?,
            role: r.get::<_, String>(2)?,
            content: r.get::<_, String>(3)?,
            created_at: r.get::<_, String>(4)?,
            seq: r.get::<_, i64>(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE conversations (
                conversation_id TEXT PRIMARY KEY,
                title           TEXT,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );
            CREATE TABLE messages (
                message_id      TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(conversation_id),
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                seq             INTEGER NOT NULL
            );",
        )
        .unwrap();
        c
    }

    #[test]
    fn create_then_append_then_list_messages() {
        let c = fresh_conn();
        let conv = create(&c, Some("First")).unwrap();
        append_message(&c, conv, Role::User, "hello").unwrap();
        append_message(&c, conv, Role::Assistant, "hi there").unwrap();
        let msgs = messages(&c, conv).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].seq, 2);
    }

    #[test]
    fn list_orders_by_updated_desc() {
        let c = fresh_conn();
        let a = create(&c, Some("A")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let b = create(&c, Some("B")).unwrap();
        // Touch A by appending, so A becomes newest.
        append_message(&c, a, Role::User, "x").unwrap();
        let convs = list(&c).unwrap();
        assert_eq!(convs[0].conversation_id, a.to_string());
        assert_eq!(convs[1].conversation_id, b.to_string());
    }

    #[test]
    fn role_round_trips() {
        for r in [Role::System, Role::User, Role::Assistant, Role::Tool] {
            assert_eq!(Role::parse(r.as_str()), Some(r));
        }
        assert_eq!(Role::parse("nope"), None);
    }
}
