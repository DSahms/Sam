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

use crate::crypto::{aead, SecretKey};
use crate::error::{AppError, AppResult};

/// Provider configuration stored per vault. Non-secret: the endpoint URL and
/// default model name. Secret credentials (API keys for Venice) will be stored
/// encrypted in a future hardening step.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_venice_endpoint")]
    pub venice_endpoint: String,
    #[serde(default)]
    pub venice_model: String,
    #[serde(default)]
    pub venice_enabled: bool,
    /// Read-only UI hint. The credential itself never crosses the Tauri boundary.
    #[serde(default)]
    pub venice_has_api_key: bool,
    /// Feature gate for read-only queries to the external PKC product.
    /// Default off. This is not Sammy's internal vault corpus.
    #[serde(default)]
    pub pkc_enabled: bool,
    /// Python executable used to run the PKC Reference Client bridge script.
    #[serde(default)]
    pub pkc_python_executable: String,
    /// Absolute path to `storykeeper_pkc_bridge.py` (filename is historical).
    #[serde(default)]
    pub pkc_bridge_script: String,
    /// Optional PKC repository root passed to the bridge.
    #[serde(default)]
    pub pkc_root: String,
    /// Canonical source ID for protected retrieval. Empty disables the query.
    #[serde(default)]
    pub pkc_source_id: String,
    /// Last verified PKC health state (no corpus text). Empty means not tested.
    #[serde(default)]
    pub pkc_last_health_state: String,
    /// RFC3339 timestamp of the last PKC probe.
    #[serde(default)]
    pub pkc_last_health_at: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            koboldcpp_endpoint: String::new(),
            koboldcpp_model: String::new(),
            koboldcpp_enabled: false,
            venice_endpoint: default_venice_endpoint(),
            venice_model: String::new(),
            venice_enabled: false,
            venice_has_api_key: false,
            pkc_enabled: false,
            pkc_python_executable: String::new(),
            pkc_bridge_script: String::new(),
            pkc_root: String::new(),
            pkc_source_id: String::new(),
            pkc_last_health_state: String::new(),
            pkc_last_health_at: String::new(),
        }
    }
}

fn default_venice_endpoint() -> String {
    "https://api.venice.ai/api/v1".into()
}

/// Ensure the settings table exists (idempotent).
pub fn ensure_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS provider_config (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            config_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
         CREATE TABLE IF NOT EXISTS provider_secrets (
            provider_id TEXT PRIMARY KEY,
            encrypted_secret BLOB NOT NULL,
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

/// Store a Venice API key encrypted with the active vault DEK. The key is not
/// included in ProviderConfig and therefore cannot be returned to React.
pub fn save_venice_api_key(
    conn: &Connection,
    dek: &SecretKey,
    api_key: &str,
) -> AppResult<()> {
    ensure_schema(conn)?;
    if api_key.trim().is_empty() {
        conn.execute(
            "DELETE FROM provider_secrets WHERE provider_id='venice'",
            [],
        )?;
        return Ok(());
    }
    let encrypted = aead::encrypt(dek, api_key.trim().as_bytes());
    conn.execute(
        "INSERT INTO provider_secrets(provider_id, encrypted_secret, updated_at)
         VALUES ('venice', ?1, ?2)
         ON CONFLICT(provider_id) DO UPDATE SET encrypted_secret=?1, updated_at=?2",
        params![encrypted, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn load_venice_api_key(
    conn: &Connection,
    dek: &SecretKey,
) -> AppResult<Option<zeroize::Zeroizing<String>>> {
    ensure_schema(conn)?;
    let mut stmt = conn.prepare(
        "SELECT encrypted_secret FROM provider_secrets WHERE provider_id='venice'",
    )?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let encrypted: Vec<u8> = row.get(0)?;
    let clear = aead::decrypt(dek, &encrypted).map_err(|_| AppError::Crypto)?;
    let value =
        String::from_utf8(clear.as_slice().to_vec()).map_err(|_| AppError::Crypto)?;
    Ok(Some(zeroize::Zeroizing::new(value)))
}

pub fn has_venice_api_key(conn: &Connection) -> AppResult<bool> {
    ensure_schema(conn)?;
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM provider_secrets WHERE provider_id='venice')",
        [],
        |row| row.get(0),
    )?)
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
            ..ProviderConfig::default()
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
            ..ProviderConfig::default()
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
        assert!(!loaded.pkc_enabled);
    }

    #[test]
    fn missing_pkc_fields_default_off() {
        let s = r#"{"koboldcpp_endpoint":"","koboldcpp_model":"","koboldcpp_enabled":false,"venice_endpoint":"https://api.venice.ai/api/v1","venice_model":"","venice_enabled":false,"venice_has_api_key":false}"#;
        let back: ProviderConfig = serde_json::from_str(s).unwrap();
        assert!(!back.pkc_enabled);
        assert!(back.pkc_bridge_script.is_empty());
        assert!(back.pkc_last_health_state.is_empty());
    }

    #[test]
    fn pkc_settings_round_trip_including_health_snapshot() {
        let conn = Connection::open_in_memory().unwrap();
        let c = ProviderConfig {
            pkc_enabled: true,
            pkc_python_executable: "python".into(),
            pkc_bridge_script: r"D:\dev\StoryKeeper\tools\storykeeper_pkc_bridge.py"
                .into(),
            pkc_root: r"F:\personal-knowledge-corpus-scaffold\personal-knowledge-corpus"
                .into(),
            pkc_source_id: "SRC-SHA256-test".into(),
            pkc_last_health_state: "available_authorized".into(),
            pkc_last_health_at: "2026-08-16T00:00:00Z".into(),
            ..ProviderConfig::default()
        };
        save(&conn, &c).unwrap();
        let loaded = load(&conn).unwrap();
        assert!(loaded.pkc_enabled);
        assert_eq!(loaded.pkc_source_id, "SRC-SHA256-test");
        assert_eq!(loaded.pkc_last_health_state, "available_authorized");
        assert!(!loaded.pkc_last_health_at.is_empty());
    }

    #[test]
    fn venice_key_is_encrypted_at_rest_and_round_trips() {
        let conn = Connection::open_in_memory().unwrap();
        let dek = SecretKey::random();
        let key = "venice-private-test-key";
        save_venice_api_key(&conn, &dek, key).unwrap();
        assert!(has_venice_api_key(&conn).unwrap());
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT encrypted_secret FROM provider_secrets WHERE provider_id='venice'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!String::from_utf8_lossy(&blob).contains(key));
        assert_eq!(
            load_venice_api_key(&conn, &dek).unwrap().unwrap().as_str(),
            key
        );
    }

    #[test]
    fn empty_venice_key_removes_secret() {
        let conn = Connection::open_in_memory().unwrap();
        let dek = SecretKey::random();
        save_venice_api_key(&conn, &dek, "key").unwrap();
        save_venice_api_key(&conn, &dek, "").unwrap();
        assert!(!has_venice_api_key(&conn).unwrap());
    }
}
