//! `settings` — per-vault application settings (Phase 2 foundation).
//!
//! Stores provider configuration (endpoint URLs, model names) per vault. In a
//! future hardening step, provider credentials (API keys) will be encrypted
//! at rest with the vault DEK; for now, local KoboldCpp needs no API key and
//! the endpoint/model are non-secret configuration.
//!
//! Directive §13: provider credentials must be encrypted at rest. KoboldCpp
//! local endpoints typically use no key, so only the URL+model are stored
//! here (both non-secret). When Venice API keys are wired, they will be
//! encrypted via AES-256-GCM with the vault DEK.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// Provider configuration stored per vault. Non-secret: the endpoint URL and
/// default model name. Secret credentials (API keys for Venice) will be stored
/// encrypted in a future hardening step.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// KoboldCpp endpoint base URL (e.g. "http://localhost:5001"). Empty if
    /// not configured.
    #[serde(default)]
    pub koboldcpp_endpoint: String,
    /// Default model name for KoboldCpp (e.g. the loaded GGUF's id). Empty if
    /// unset; the provider will query `/v1/models` to list available.
    #[serde(default)]
    pub koboldcpp_model: String,
    /// Whether the KoboldCpp provider is enabled.
    #[serde(default)]
    pub koboldcpp_enabled: bool,
}

/// Ensure the settings table exists (idempotent).
pub fn ensure_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS provider_config (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            config_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

/// Load the provider config for the active vault, or the default if unset.
pub fn load(conn: &Connection) -> AppResult<ProviderConfig> {
    ensure_schema(conn)?;
    let exists: i64 = conn.query_row(
        "SELECT count(*) FROM provider_config WHERE singleton = 1",
        [],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Ok(ProviderConfig::default());
    }
    let json: String = conn.query_row(
        "SELECT config_json FROM provider_config WHERE singleton = 1",
        [],
        |r| r.get(0),
    )?;
    let config: ProviderConfig = serde_json::from_str(&json).unwrap_or_default();
    Ok(config)
}

/// Save the provider config (upsert).
pub fn save(conn: &Connection, config: &ProviderConfig) -> AppResult<()> {
    ensure_schema(conn)?;
    let json = serde_json::to_string(config)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO provider_config (singleton, config_json, updated_at)
         VALUES (1, ?1, ?2)
         ON CONFLICT(singleton) DO UPDATE SET config_json = ?1, updated_at = ?2",
        params![json, now],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips() {
        let c = ProviderConfig {
            koboldcpp_endpoint: "http://localhost:5001".into(),
            koboldcpp_model: "test-model".into(),
            koboldcpp_enabled: true,
        };
        let s = serde_json::to_string(&c).unwrap();
        let back: ProviderConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.koboldcpp_endpoint, "http://localhost:5001");
        assert!(back.koboldcpp_enabled);
    }

    #[test]
    fn store_round_trips_in_db() {
        let conn = Connection::open_in_memory().unwrap();
        let c = ProviderConfig {
            koboldcpp_endpoint: "http://localhost:9999".into(),
            koboldcpp_model: "m1".into(),
            koboldcpp_enabled: true,
        };
        save(&conn, &c).unwrap();
        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.koboldcpp_endpoint, "http://localhost:9999");
        assert_eq!(loaded.koboldcpp_model, "m1");
        assert!(loaded.koboldcpp_enabled);
    }

    #[test]
    fn load_returns_default_when_empty() {
        let conn = Connection::open_in_memory().unwrap();
        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.koboldcpp_endpoint, "");
        assert!(!loaded.koboldcpp_enabled);
    }
}
