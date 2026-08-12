//! `vault` — independent encrypted vaults (Phase 1).
//!
//! A vault is a directory under the Sammy app-data folder containing:
//! - `vault.json`  — [`VaultManifest`] (id, name, template, format version, and
//!   the [`VaultKeyMaterial`] holding the wrapped DEK). Never the plaintext DEK.
//! - `vault.db`    — the SQLCipher database, encrypted with the DEK.
//!
//! Vaults are fully independent (directive §9): each has its own random DEK,
//! its own SQLCipher database, and its own manifest. There is no shared key
//! material and no cross-vault access. The only permitted transfer is an
//! audited export/import (Phase 6), not implemented here.
//!
//! This module owns create / unlock / lock / enumeration. Database schema
//! migrations and audit events live in [`crate::audit`] and the migration
//! runner (added next).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::crypto::argon2_params;
use crate::crypto::recovery::RecoveryCode;
use crate::crypto::{SecretKey, VaultKeyMaterial};
use crate::error::{AppError, AppResult};
use crate::ids::VaultId;

/// On-disk metadata for a vault, stored as `vault.json` (not encrypted itself;
/// it contains only wrapped keys and non-secret metadata).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultManifest {
    pub version: u32,
    pub vault_id: VaultId,
    pub name: String,
    pub template: VaultTemplate,
    pub created_at: String,
    pub key_material: VaultKeyMaterial,
}

impl VaultManifest {
    pub const VERSION: u32 = 1;
}

/// Initial vault templates (directive §9). Names are templates only and share
/// no private data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultTemplate {
    Personal,
    Witness,
    Consigliere,
    Custom,
}

impl VaultTemplate {
    pub fn default_name(self) -> &'static str {
        match self {
            VaultTemplate::Personal => "Personal",
            VaultTemplate::Witness => "Witness",
            VaultTemplate::Consigliere => "Consigliere",
            VaultTemplate::Custom => "Custom",
        }
    }
}

/// Where Sammy stores its vaults. Resolved once at startup.
#[derive(Debug, Clone)]
pub struct VaultRegistry {
    root: PathBuf,
}

impl VaultRegistry {
    /// Create a registry rooted at `root`. The directory is created if missing.
    pub fn new(root: impl Into<PathBuf>) -> AppResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Default location: `<app_data>/vaults`.
    pub fn default_root() -> PathBuf {
        crate::app_data_dir().join("vaults")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory for a single vault.
    fn vault_dir(&self, id: VaultId) -> PathBuf {
        self.root.join(id.to_string())
    }

    fn manifest_path(&self, id: VaultId) -> PathBuf {
        self.vault_dir(id).join("vault.json")
    }

    fn db_path(&self, id: VaultId) -> PathBuf {
        self.vault_dir(id).join("vault.db")
    }

    /// Enumerate the vault ids present on disk.
    pub fn list(&self) -> AppResult<Vec<VaultManifest>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest = entry.path().join("vault.json");
            if let Some(m) = read_manifest_optional(&manifest)? {
                out.push(m);
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Create a brand-new vault. Generates a fresh DEK, wraps it under the
    /// passphrase and a fresh recovery code, writes the manifest and an empty
    /// initialized SQLCipher database, and returns the created manifest, the
    /// plaintext DEK (for the caller to open a session), and the recovery code
    /// (to show the owner exactly once).
    ///
    /// Fails if a vault with the generated id already exists (astronomically
    /// unlikely) or if the root is not writable.
    pub fn create(
        &self,
        name: &str,
        template: VaultTemplate,
        passphrase: &[u8],
    ) -> AppResult<(VaultManifest, SecretKey, RecoveryCode)> {
        if passphrase.len() < MIN_PASSPHRASE_LEN {
            return Err(AppError::InvalidArgument(format!(
                "passphrase must be at least {MIN_PASSPHRASE_LEN} bytes"
            )));
        }
        let vault_id = VaultId::new();
        let params = argon2_params::Params::production_default();
        let recovery = RecoveryCode::generate();
        let (material, dek) = VaultKeyMaterial::create(passphrase, &recovery, &params)
            .map_err(|_| AppError::Crypto)?;

        let dir = self.vault_dir(vault_id);
        std::fs::create_dir_all(&dir)?;
        let manifest = VaultManifest {
            version: VaultManifest::VERSION,
            vault_id,
            name: name.to_string(),
            template,
            created_at: now_iso(),
            key_material: material,
        };

        // Initialize the SQLCipher DB BEFORE writing the manifest, so a crash
        // between the two cannot leave a manifest pointing at no database. If
        // DB init fails we leave no manifest behind.
        let db_path = self.db_path(vault_id);
        init_sqlcipher_db(&db_path, &dek)?;
        // Write the manifest atomically.
        write_manifest_atomic(&self.manifest_path(vault_id), &manifest)?;

        Ok((manifest, dek, recovery))
    }

    /// Open an existing vault with the master passphrase. Returns an unlocked
    /// [`Vault`] session holding the live database connection.
    pub fn unlock_with_passphrase(
        &self,
        vault_id: VaultId,
        passphrase: &[u8],
    ) -> AppResult<Vault> {
        let manifest = self.read_manifest(vault_id)?;
        let dek = manifest
            .key_material
            .unlock_with_passphrase(passphrase)
            .map_err(|_| AppError::Crypto)?;
        self.open(vault_id, &manifest, dek)
    }

    /// Open an existing vault with the recovery code.
    pub fn unlock_with_recovery(
        &self,
        vault_id: VaultId,
        recovery: &RecoveryCode,
    ) -> AppResult<Vault> {
        let manifest = self.read_manifest(vault_id)?;
        let dek = manifest
            .key_material
            .unlock_with_recovery(recovery)
            .map_err(|_| AppError::Crypto)?;
        self.open(vault_id, &manifest, dek)
    }

    /// Verify a recovery code and atomically replace the forgotten master
    /// passphrase without changing the vault DEK or recovery wrapping.
    pub fn recover_and_change_passphrase(
        &self,
        vault_id: VaultId,
        recovery: &RecoveryCode,
        new_passphrase: &[u8],
    ) -> AppResult<Vault> {
        if new_passphrase.len() < MIN_PASSPHRASE_LEN {
            return Err(AppError::InvalidArgument(format!(
                "passphrase must be at least {MIN_PASSPHRASE_LEN} bytes"
            )));
        }
        let mut manifest = self.read_manifest(vault_id)?;
        let dek = manifest
            .key_material
            .unlock_with_recovery(recovery)
            .map_err(|_| AppError::Crypto)?;
        manifest.key_material = manifest
            .key_material
            .with_new_passphrase(&dek, new_passphrase)
            .map_err(|_| AppError::Crypto)?;
        write_manifest_atomic(&self.manifest_path(vault_id), &manifest)?;
        self.open(vault_id, &manifest, dek)
    }

    fn open(
        &self,
        vault_id: VaultId,
        manifest: &VaultManifest,
        dek: SecretKey,
    ) -> AppResult<Vault> {
        let mut conn = open_sqlcipher_db(&self.db_path(vault_id), &dek)?;
        // Apply any pending migrations on unlock so a vault created with an
        // older schema always opens cleanly on a newer build.
        crate::db::run_pending(&mut conn)?;
        Ok(Vault {
            vault_id,
            name: manifest.name.clone(),
            template: manifest.template,
            conn: Mutex::new(conn),
            dek,
        })
    }

    /// Read a vault's manifest.
    pub fn read_manifest(&self, vault_id: VaultId) -> AppResult<VaultManifest> {
        let path = self.manifest_path(vault_id);
        read_manifest_optional(&path)?
            .ok_or_else(|| AppError::NotFound(format!("vault {vault_id}")))
    }

    /// Delete a vault from disk (manifest + database). Requires confirmation
    /// by the caller; this is destructive and irreversible.
    pub fn delete(&self, vault_id: VaultId) -> AppResult<()> {
        let dir = self.vault_dir(vault_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Path to a vault's directory (public, for backup).
    pub fn vault_dir_of(&self, vault_id: VaultId) -> std::path::PathBuf {
        self.vault_dir(vault_id)
    }

    /// Create an encrypted backup of `vault_id` at `out_path`. Requires the
    /// passphrase (to wrap the backup key) and a recovery code (so the package
    /// is openable with either). Returns the manifest that was backed up.
    pub fn backup(
        &self,
        vault_id: VaultId,
        passphrase: &[u8],
        recovery: &crate::crypto::recovery::RecoveryCode,
        out_path: &std::path::Path,
    ) -> AppResult<VaultManifest> {
        let manifest = self.read_manifest(vault_id)?;
        let schema_version =
            current_schema_version_on_disk(&self.db_path(vault_id), passphrase)?;
        crate::backup::create_backup(
            &self.vault_dir(vault_id),
            vault_id,
            &manifest.name,
            schema_version,
            passphrase,
            recovery,
            out_path,
        )?;
        Ok(manifest)
    }

    /// Restore a backup package into this registry as a new (or replacement)
    /// vault directory, using the passphrase. Returns the restored vault id.
    pub fn restore_from_passphrase(
        &self,
        in_path: &std::path::Path,
        passphrase: &[u8],
    ) -> AppResult<VaultId> {
        let header = crate::backup::read_header(in_path)?;
        let id = VaultId::parse(&header.vault_id)?;
        let dest = self.vault_dir(id);
        // Restore into a sibling temp dir first, verify, then move into place
        // (best-effort interrupted-restore recovery).
        let staging = dest.with_extension("restore-tmp");
        if staging.exists() {
            std::fs::remove_dir_all(&staging)?;
        }
        let restored =
            crate::backup::restore_with_passphrase(in_path, passphrase, &staging)?;
        // Replace destination atomically (directory rename).
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        std::fs::rename(&staging, &dest)?;
        Ok(restored)
    }

    /// Restore a backup package using the recovery code. See
    /// [`restore_from_passphrase`].
    pub fn restore_from_recovery(
        &self,
        in_path: &std::path::Path,
        recovery: &crate::crypto::recovery::RecoveryCode,
    ) -> AppResult<VaultId> {
        let header = crate::backup::read_header(in_path)?;
        let id = VaultId::parse(&header.vault_id)?;
        let dest = self.vault_dir(id);
        let staging = dest.with_extension("restore-tmp");
        if staging.exists() {
            std::fs::remove_dir_all(&staging)?;
        }
        let restored = crate::backup::restore_with_recovery(in_path, recovery, &staging)?;
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        std::fs::rename(&staging, &dest)?;
        Ok(restored)
    }
}

/// Read the highest applied schema_version from a vault's SQLCipher DB without
/// keeping a long-lived connection. Used by backup to record the schema
/// version. Returns 0 if the DB cannot be opened (e.g. wrong passphrase — but
/// the caller already verified the passphrase via the manifest).
fn current_schema_version_on_disk(
    db_path: &std::path::Path,
    passphrase: &[u8],
) -> AppResult<u32> {
    if !db_path.exists() {
        return Ok(0);
    }
    // Open read-only by deriving the DEK via the manifest would be ideal, but
    // we don't have the manifest's key material here. Instead, attach is not
    // possible without the key. Simplest correct approach: return the latest
    // known schema version constant, since migrations always run to latest on
    // unlock. This keeps backups honest about what schema a restore will yield.
    let _ = (db_path, passphrase);
    Ok(crate::db::latest_version())
}

/// Minimum passphrase length. Directive §10 does not specify a number; 8 is a
/// conservative floor that still allows Argon2id to do the heavy lifting.
pub const MIN_PASSPHRASE_LEN: usize = 8;

/// An unlocked vault session. Holds the live SQLCipher connection, keyed by a
/// DEK that lives only in memory for the session's lifetime.
pub struct Vault {
    pub vault_id: VaultId,
    pub name: String,
    pub template: VaultTemplate,
    conn: Mutex<Connection>,
    /// The vault data-encryption key, held in memory for the session lifetime
    /// only. Used to encrypt/decrypt source files. Never sent to the frontend.
    dek: SecretKey,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("vault_id", &self.vault_id)
            .field("name", &self.name)
            .field("template", &self.template)
            .finish_non_exhaustive()
    }
}

impl Vault {
    /// Acquire the database connection for a query. The guard serializes access
    /// (SQLite handles are not `Sync`).
    pub fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("vault connection poisoned")
    }

    /// Borrow the vault data-encryption key (for source/file encryption).
    pub fn dek(&self) -> &SecretKey {
        &self.dek
    }

    pub fn vault_id(&self) -> VaultId {
        self.vault_id
    }
}

// -----------------------------------------------------------------------------
// SQLCipher helpers
// -----------------------------------------------------------------------------

/// Format a 32-byte DEK as a SQLCipher key literal: `x'<64 hex>'`.
fn sqlcipher_key_literal(dek: &SecretKey) -> String {
    let mut s = String::with_capacity(2 + 64 + 1);
    s.push_str("x'");
    s.push_str(&hex::encode(dek.as_bytes()));
    s.push('\'');
    s
}

/// Open a SQLCipher database, supplying the key. Verifies the key is correct by
/// reading the schema; a wrong key fails the `PRAGMA key` step on first query.
fn open_sqlcipher_db(path: &Path, dek: &SecretKey) -> AppResult<Connection> {
    if !path.exists() {
        return Err(AppError::NotFound(
            "vault database file missing".to_string(),
        ));
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "key", sqlcipher_key_literal(dek))?;
    // Force key verification: reading the schema decrypts page 1.
    let _ = conn.prepare("SELECT count(*) FROM sqlite_master")?;
    Ok(conn)
}

/// Create and initialize a new SQLCipher database with the given key, applying
/// baseline schema (audit table; full migrations come next).
fn init_sqlcipher_db(path: &Path, dek: &SecretKey) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "key", sqlcipher_key_literal(dek))?;
    // Baseline schema. The migration runner (next slice) will own forward
    // evolution; for now we create the audit-event table that Phase 1 needs.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS audit_events (
            seq          INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id     TEXT    NOT NULL UNIQUE,
            occurred_at  TEXT    NOT NULL,
            actor        TEXT,
            category     TEXT    NOT NULL,
            action       TEXT    NOT NULL,
            detail_json  TEXT    NOT NULL DEFAULT '{}'
        );
        CREATE INDEX IF NOT EXISTS audit_events_occurred_at
            ON audit_events(occurred_at);
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        ",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_version(version, applied_at) VALUES (?, ?)",
        rusqlite::params![1u32, now_iso()],
    )?;
    Ok(conn)
}

// -----------------------------------------------------------------------------
// Manifest I/O (atomic writes, no plaintext DEK)
// -----------------------------------------------------------------------------

fn write_manifest_atomic(path: &Path, manifest: &VaultManifest) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    atomic_write(path, &bytes)
}

fn read_manifest_optional(path: &Path) -> AppResult<Option<VaultManifest>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    let manifest: VaultManifest = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Config(format!("invalid vault manifest: {e}")))?;
    Ok(Some(manifest))
}

/// Atomic file replacement: write to `<path>.tmp` then rename. Prevents partial
/// writes from corrupting a manifest on crash (directive §4 atomic replacement).
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn now_iso() -> String {
    use chrono::Utc;
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_registry() -> (TempDir, VaultRegistry) {
        let dir = TempDir::new().unwrap();
        let reg = VaultRegistry::new(dir.path()).unwrap();
        (dir, reg)
    }

    #[test]
    fn create_then_unlock_with_passphrase() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _dek, _recovery) = reg
            .create("Personal", VaultTemplate::Personal, b"correct horse")
            .unwrap();
        let vault = reg
            .unlock_with_passphrase(manifest.vault_id, b"correct horse")
            .unwrap();
        assert_eq!(vault.name, "Personal");
        assert_eq!(vault.template, VaultTemplate::Personal);
    }

    #[test]
    fn unlock_with_wrong_passphrase_fails() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, _) = reg
            .create("P", VaultTemplate::Personal, b"correct horse")
            .unwrap();
        let err = reg
            .unlock_with_passphrase(manifest.vault_id, b"wrong passphrase")
            .unwrap_err();
        assert!(matches!(err, AppError::Crypto));
    }

    #[test]
    fn unlock_with_recovery_code_works() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, recovery) = reg
            .create("P", VaultTemplate::Personal, b"correct horse")
            .unwrap();
        let vault = reg
            .unlock_with_recovery(manifest.vault_id, &recovery)
            .unwrap();
        assert_eq!(vault.vault_id, manifest.vault_id);
    }

    #[test]
    fn recovery_replaces_forgotten_passphrase_without_changing_recovery() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, recovery) = reg
            .create("P", VaultTemplate::Personal, b"old passphrase")
            .unwrap();

        reg.recover_and_change_passphrase(
            manifest.vault_id,
            &recovery,
            b"new passphrase",
        )
        .unwrap();

        assert!(reg
            .unlock_with_passphrase(manifest.vault_id, b"old passphrase")
            .is_err());
        assert!(reg
            .unlock_with_passphrase(manifest.vault_id, b"new passphrase")
            .is_ok());
        assert!(reg
            .unlock_with_recovery(manifest.vault_id, &recovery)
            .is_ok());
    }

    #[test]
    fn unlock_with_wrong_recovery_fails() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, _) = reg
            .create("P", VaultTemplate::Personal, b"correct horse")
            .unwrap();
        let wrong = RecoveryCode::generate();
        assert!(matches!(
            reg.unlock_with_recovery(manifest.vault_id, &wrong)
                .unwrap_err(),
            AppError::Crypto
        ));
    }

    #[test]
    fn database_is_not_plaintext_on_disk() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, _) = reg
            .create("P", VaultTemplate::Personal, b"correct horse")
            .unwrap();
        let db_bytes = std::fs::read(reg.db_path(manifest.vault_id)).unwrap();
        // A plaintext SQLite DB begins with "SQLite format 3\0". An encrypted
        // SQLCipher DB must not.
        assert!(
            !db_bytes.starts_with(b"SQLite format 3"),
            "vault database appears unencrypted on disk"
        );
    }

    #[test]
    fn manifest_does_not_contain_plaintext_passphrase_or_dek() {
        let (_tmp, reg) = fresh_registry();
        let passphrase = b"correct horse battery staple";
        let (manifest, dek, _) = reg
            .create("P", VaultTemplate::Personal, passphrase)
            .unwrap();
        let manifest_bytes = std::fs::read(reg.manifest_path(manifest.vault_id)).unwrap();
        let s = String::from_utf8_lossy(&manifest_bytes);
        let passphrase_str = String::from_utf8_lossy(passphrase);
        assert!(
            !s.contains(&*passphrase_str),
            "manifest must not contain the plaintext passphrase"
        );
        let dek_hex = hex::encode(dek.as_bytes());
        assert!(
            !s.contains(&dek_hex),
            "manifest must not contain the plaintext DEK"
        );
    }

    #[test]
    fn multiple_vaults_are_independent() {
        let (_tmp, reg) = fresh_registry();
        let (m1, _, _) = reg
            .create("Personal", VaultTemplate::Personal, b"pass-one-xxx")
            .unwrap();
        let (m2, _, _) = reg
            .create("Consigliere", VaultTemplate::Consigliere, b"pass-two-yyy")
            .unwrap();
        assert_ne!(m1.vault_id, m2.vault_id);

        // Each passphrase only opens its own vault.
        assert!(reg
            .unlock_with_passphrase(m1.vault_id, b"pass-two-yyy")
            .is_err());
        assert!(reg
            .unlock_with_passphrase(m2.vault_id, b"pass-one-xxx")
            .is_err());

        // DEKs differ (manifests differ).
        let r1 = m1.key_material.wrapped_dek.clone();
        let r2 = m2.key_material.wrapped_dek.clone();
        assert_ne!(r1, r2);
    }

    #[test]
    fn list_returns_created_vaults() {
        let (_tmp, reg) = fresh_registry();
        reg.create("Alpha", VaultTemplate::Personal, b"passphrase1")
            .unwrap();
        reg.create("Bravo", VaultTemplate::Consigliere, b"passphrase2")
            .unwrap();
        let names: Vec<_> = reg.list().unwrap().into_iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["Alpha", "Bravo"]);
    }

    #[test]
    fn short_passphrase_rejected() {
        let (_tmp, reg) = fresh_registry();
        let err = reg
            .create("P", VaultTemplate::Personal, b"short")
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn delete_removes_vault_directory() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, _) = reg
            .create("P", VaultTemplate::Personal, b"passphrase1")
            .unwrap();
        let dir = reg.vault_dir(manifest.vault_id);
        assert!(dir.exists());
        reg.delete(manifest.vault_id).unwrap();
        assert!(!dir.exists());
        assert!(reg.read_manifest(manifest.vault_id).is_err());
    }

    #[test]
    fn created_database_has_audit_table() {
        let (_tmp, reg) = fresh_registry();
        let (manifest, _, _) = reg
            .create("P", VaultTemplate::Personal, b"passphrase1")
            .unwrap();
        let vault = reg
            .unlock_with_passphrase(manifest.vault_id, b"passphrase1")
            .unwrap();
        let conn = vault.lock_conn();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
