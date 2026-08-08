//! `backup` — encrypted backup and restore (Phase 1).
//!
//! Directive §30/§31. A backup is an opaque encrypted package written to a
//! user-selected location. It must restore on a replacement computer using
//! only the master passphrase or recovery key — without the original OS
//! account, Windows keychain, the original computer, or an online service.
//!
//! # Package format (v1)
//! A Sammy backup file is a length-prefixed binary container:
//!
//! ```text
//! magic            : 8 bytes   b"SAMMYBK1"
//! header_len       : 4 bytes   big-endian u32
//! header_json      : header_len bytes  (BackupHeader, including wrapped
//!                                       backup-key, Argon2id params, salts,
//!                                       checksums, manifest of contents)
//! body_len         : 8 bytes   big-endian u64
//! body             : body_len bytes    AES-256-GCM ciphertext of the
//!                                       serialized BackupBody
//! ```
//!
//! The body is encrypted with a fresh random 256-bit backup data key (BDK).
//! The BDK is wrapped by both the passphrase KEK (Argon2id of the passphrase)
//! and the recovery KEK (HKDF of the recovery code), mirroring the vault key
//! wrapping so the package can be opened with either credential. The BDK is
//! never the vault DEK — restoring simply places the encrypted vault files
//! back on disk, after which the existing vault unlock path (with the DEK
//! still inside `vault.db`) works.
//!
//! Checksums (SHA-256) of each included file and of the whole body are stored
//! in the header and verified on restore.

use std::path::Path;

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::crypto::argon2_params;
use crate::crypto::recovery::RecoveryCode;
use crate::crypto::{aead, random, unwrap_key, wrap_key, CryptoError, SecretKey};
use crate::error::{AppError, AppResult};
use crate::ids::VaultId;

const MAGIC: &[u8; 8] = b"SAMMYBK1";
const BACKUP_FORMAT_VERSION: u32 = 1;

/// One file entry inside the backup body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    /// Path relative to the vault directory, using forward slashes.
    pub rel_path: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// The cleartext body of a backup: the set of files plus a small manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBody {
    pub vault_id: String,
    pub app_version: String,
    pub schema_version: u32,
    pub files: Vec<BackupFile>,
}

/// The on-disk header. Carries wrapped BDK material + checksums but never a
/// plaintext key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupHeader {
    pub format_version: u32,
    pub vault_id: String,
    pub vault_name: String,
    pub created_at: String,
    /// Argon2id salt for the passphrase KEK.
    pub passphrase_salt: [u8; 16],
    /// Argon2id parameters used for the passphrase KEK.
    pub argon2_params: argon2_params::Params,
    /// BDK wrapped under the passphrase KEK (base64).
    pub wrapped_bdk_passphrase: String,
    /// BDK wrapped under the recovery KEK (base64).
    pub wrapped_bdk_recovery: String,
    /// SHA-256 of the encrypted body, for transport-integrity checks.
    pub body_sha256: String,
    /// SHA-256 of the cleartext body (recorded before encryption; verified
    /// after decryption). Belt-and-suspenders.
    pub plaintext_body_sha256: String,
}

/// Build a backup package for the vault at `vault_dir`, encrypting it under the
/// given passphrase (and a recovery code, so either can restore it). Writes the
/// package to `out_path`.
///
/// `schema_version` is the highest migration version recorded for the vault.
pub fn create_backup(
    vault_dir: &Path,
    vault_id: VaultId,
    vault_name: &str,
    schema_version: u32,
    passphrase: &[u8],
    recovery: &RecoveryCode,
    out_path: &Path,
) -> AppResult<()> {
    let body = collect_body(vault_dir, vault_id, vault_name, schema_version)?;
    let plaintext = serde_json::to_vec(&body)?;
    let plaintext_sha = hex::encode(Sha256::digest(&plaintext));

    // Fresh backup data key; wrap under passphrase KEK + recovery KEK.
    let bdk = SecretKey::random();
    let pass_salt = random::salt16();
    let params = argon2_params::Params::production_default();
    let pass_kek = crate::crypto::derive_kek(passphrase, &pass_salt, &params)
        .map_err(|_| AppError::Crypto)?;
    let rec_kek = crate::crypto::recovery::derive_recovery_kek(recovery, &pass_salt)
        .map_err(|_| AppError::Crypto)?;
    let wrapped_pass = wrap_key(&pass_kek, &bdk);
    let wrapped_rec = wrap_key(&rec_kek, &bdk);

    let ciphertext = aead::encrypt(&bdk, &plaintext);
    let body_sha = hex::encode(Sha256::digest(&ciphertext));

    let header = BackupHeader {
        format_version: BACKUP_FORMAT_VERSION,
        vault_id: vault_id.to_string(),
        vault_name: vault_name.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        passphrase_salt: pass_salt,
        argon2_params: params,
        wrapped_bdk_passphrase: b64().encode(&wrapped_pass),
        wrapped_bdk_recovery: b64().encode(&wrapped_rec),
        body_sha256: body_sha,
        plaintext_body_sha256: plaintext_sha,
    };
    let header_bytes = serde_json::to_vec(&header)?;

    let mut out = Vec::with_capacity(8 + 4 + header_bytes.len() + 8 + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&(ciphertext.len() as u64).to_be_bytes());
    out.extend_from_slice(&ciphertext);

    crate::vault::atomic_write(out_path, &out)?;
    Ok(())
}

/// Restore a backup package from `in_path` into `dest_vault_dir`, using the
/// master passphrase. Verifies checksums before writing. Returns the restored
/// vault id.
pub fn restore_with_passphrase(
    in_path: &Path,
    passphrase: &[u8],
    dest_vault_dir: &Path,
) -> AppResult<VaultId> {
    let (header, body) = read_and_decrypt(in_path, |h| {
        let kek =
            crate::crypto::derive_kek(passphrase, &h.passphrase_salt, &h.argon2_params)
                .map_err(|_| CryptoError::VerifyFailed)?;
        let blob = b64()
            .decode(&h.wrapped_bdk_passphrase)
            .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        unwrap_key(&kek, &blob)
    })?;
    install_body(dest_vault_dir, &header, &body)
}

/// Restore a backup package using the recovery code.
pub fn restore_with_recovery(
    in_path: &Path,
    recovery: &RecoveryCode,
    dest_vault_dir: &Path,
) -> AppResult<VaultId> {
    let (header, body) = read_and_decrypt(in_path, |h| {
        let kek =
            crate::crypto::recovery::derive_recovery_kek(recovery, &h.passphrase_salt)
                .map_err(|_| CryptoError::VerifyFailed)?;
        let blob = b64()
            .decode(&h.wrapped_bdk_recovery)
            .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        unwrap_key(&kek, &blob)
    })?;
    install_body(dest_vault_dir, &header, &body)
}

/// Read the (non-secret) header of a backup without decrypting, for restore
/// preview in the UI (shows vault name, id, created date).
pub fn read_header(in_path: &Path) -> AppResult<BackupHeader> {
    let bytes = std::fs::read(in_path)?;
    parse_header(&bytes).map(|(h, _)| h)
}

/// Verify a backup file is well-formed and its body checksum matches, without
/// needing credentials. Returns Ok(()) if structurally valid.
pub fn verify_integrity(in_path: &Path) -> AppResult<()> {
    let bytes = std::fs::read(in_path)?;
    let (header, body_ct) = parse_header(&bytes)?;
    let actual = hex::encode(Sha256::digest(&body_ct));
    if actual != header.body_sha256 {
        return Err(AppError::Vault("backup body checksum mismatch".into()));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Internals
// -----------------------------------------------------------------------------

fn collect_body(
    vault_dir: &Path,
    vault_id: VaultId,
    _vault_name: &str,
    schema_version: u32,
) -> AppResult<BackupBody> {
    let mut files = Vec::new();
    for entry in walkdir(vault_dir)? {
        let rel = entry
            .strip_prefix(vault_dir)
            .map_err(|e| AppError::Config(format!("path error: {e}")))?;
        let rel_path = rel.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(&entry)?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        files.push(BackupFile {
            rel_path,
            bytes,
            sha256,
        });
    }
    Ok(BackupBody {
        vault_id: vault_id.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version,
        files,
    })
}

fn read_and_decrypt<F>(
    in_path: &Path,
    unlock_bdk: F,
) -> AppResult<(BackupHeader, BackupBody)>
where
    F: FnOnce(&BackupHeader) -> Result<SecretKey, CryptoError>,
{
    let bytes = std::fs::read(in_path)?;
    let (header, body_ct) = parse_header(&bytes)?;

    // Transport integrity first (no key needed).
    let body_sha = hex::encode(Sha256::digest(&body_ct));
    if body_sha != header.body_sha256 {
        return Err(AppError::Vault("backup body checksum mismatch".into()));
    }

    // Now decrypt. A wrong credential yields VerifyFailed (opaque).
    let bdk = unlock_bdk(&header).map_err(|_| AppError::Crypto)?;
    let plaintext = aead::decrypt(&bdk, &body_ct).map_err(|_| AppError::Crypto)?;
    let pt_sha = hex::encode(Sha256::digest(plaintext.as_slice()));
    if pt_sha != header.plaintext_body_sha256 {
        return Err(AppError::Vault("backup plaintext checksum mismatch".into()));
    }
    let body: BackupBody = serde_json::from_slice(plaintext.as_slice())
        .map_err(|e| AppError::Config(format!("invalid backup body: {e}")))?;
    for f in &body.files {
        let h = hex::encode(Sha256::digest(&f.bytes));
        if h != f.sha256 {
            return Err(AppError::Vault(format!(
                "file {} checksum mismatch",
                f.rel_path
            )));
        }
    }
    Ok((header, body))
}

fn install_body(
    dest_vault_dir: &Path,
    header: &BackupHeader,
    body: &BackupBody,
) -> AppResult<VaultId> {
    if body.vault_id != header.vault_id {
        return Err(AppError::Vault("backup vault id mismatch".into()));
    }
    let vault_id = VaultId::parse(&body.vault_id)?;

    std::fs::create_dir_all(dest_vault_dir)?;
    for f in &body.files {
        let rel = std::path::PathBuf::from(
            f.rel_path.replace('/', std::path::MAIN_SEPARATOR_STR),
        );
        let dest = dest_vault_dir.join(&rel);
        // Refuse path traversal outside the vault dir. Canonicalize both (the
        // dest's parent must exist for canonicalize, so check components first).
        if rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(AppError::Vault(format!(
                "backup entry escapes vault dir: {}",
                f.rel_path
            )));
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::vault::atomic_write(&dest, &f.bytes)?;
    }
    Ok(vault_id)
}

fn parse_header(bytes: &[u8]) -> AppResult<(BackupHeader, Vec<u8>)> {
    if bytes.len() < 8 + 4 + 8 {
        return Err(AppError::Vault("backup file too short".into()));
    }
    if &bytes[0..8] != MAGIC {
        return Err(AppError::Vault("not a Sammy backup (bad magic)".into()));
    }
    let header_len =
        u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let header_end = 8 + 4 + header_len;
    if bytes.len() < header_end + 8 {
        return Err(AppError::Vault("backup file truncated".into()));
    }
    let header: BackupHeader = serde_json::from_slice(&bytes[12..header_end])
        .map_err(|e| AppError::Config(format!("invalid backup header: {e}")))?;
    let body_len = u64::from_be_bytes([
        bytes[header_end],
        bytes[header_end + 1],
        bytes[header_end + 2],
        bytes[header_end + 3],
        bytes[header_end + 4],
        bytes[header_end + 5],
        bytes[header_end + 6],
        bytes[header_end + 7],
    ]) as usize;
    let body_start = header_end + 8;
    if bytes.len() < body_start + body_len {
        return Err(AppError::Vault("backup body truncated".into()));
    }
    let body_ct = bytes[body_start..body_start + body_len].to_vec();
    Ok((header, body_ct))
}

fn walkdir(dir: &Path) -> AppResult<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn populated_vault() -> (TempDir, VaultId) {
        let dir = TempDir::new().unwrap();
        let id = VaultId::new();
        std::fs::write(dir.path().join("vault.json"), b"{\"manifest\":1}").unwrap();
        std::fs::write(
            dir.path().join("vault.db"),
            b"SQLCipher-bytes-not-plaintext-here",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("sources")).unwrap();
        std::fs::write(dir.path().join("sources/a.txt"), b"hello world").unwrap();
        (dir, id)
    }

    #[test]
    fn backup_round_trips_with_passphrase() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("backup.sammy-backup");
        let recovery = RecoveryCode::generate();
        create_backup(
            src.path(),
            id,
            "Personal",
            4,
            b"correct horse",
            &recovery,
            &pkg,
        )
        .unwrap();

        verify_integrity(&pkg).unwrap();
        let h = read_header(&pkg).unwrap();
        assert_eq!(h.vault_name, "Personal");
        assert_eq!(h.vault_id, id.to_string());

        let dest = TempDir::new().unwrap();
        let restored =
            restore_with_passphrase(&pkg, b"correct horse", dest.path()).unwrap();
        assert_eq!(restored, id);
        assert_eq!(
            std::fs::read(dest.path().join("vault.json")).unwrap(),
            b"{\"manifest\":1}".to_vec()
        );
        assert_eq!(
            std::fs::read(dest.path().join("sources/a.txt")).unwrap(),
            b"hello world".to_vec()
        );
    }

    #[test]
    fn backup_round_trips_with_recovery() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        let recovery = RecoveryCode::generate();
        create_backup(src.path(), id, "P", 4, b"correct horse", &recovery, &pkg).unwrap();
        let dest = TempDir::new().unwrap();
        let restored = restore_with_recovery(&pkg, &recovery, dest.path()).unwrap();
        assert_eq!(restored, id);
        assert!(dest.path().join("vault.db").exists());
    }

    #[test]
    fn restore_with_wrong_passphrase_fails() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        let recovery = RecoveryCode::generate();
        create_backup(src.path(), id, "P", 4, b"correct horse", &recovery, &pkg).unwrap();
        let dest = TempDir::new().unwrap();
        assert!(matches!(
            restore_with_passphrase(&pkg, b"wrong passphrase", dest.path()).unwrap_err(),
            AppError::Crypto
        ));
    }

    #[test]
    fn restore_with_wrong_recovery_fails() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        let good = RecoveryCode::generate();
        create_backup(src.path(), id, "P", 4, b"correct horse", &good, &pkg).unwrap();
        let wrong = RecoveryCode::generate();
        let dest = TempDir::new().unwrap();
        assert!(restore_with_recovery(&pkg, &wrong, dest.path()).is_err());
    }

    #[test]
    fn verify_detects_tampered_body() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        let recovery = RecoveryCode::generate();
        create_backup(src.path(), id, "P", 4, b"correct horse", &recovery, &pkg).unwrap();
        let mut bytes = std::fs::read(&pkg).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&pkg, bytes).unwrap();
        assert!(verify_integrity(&pkg).is_err());
    }

    #[test]
    fn bad_magic_rejected() {
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        std::fs::write(&pkg, b"NOTSAMMYextra").unwrap();
        assert!(matches!(read_header(&pkg).unwrap_err(), AppError::Vault(_)));
    }

    #[test]
    fn header_does_not_contain_plaintext_passphrase() {
        let (src, id) = populated_vault();
        let out = TempDir::new().unwrap();
        let pkg = out.path().join("b");
        let recovery = RecoveryCode::generate();
        let pass = b"a-very-secret-passphrase";
        create_backup(src.path(), id, "P", 4, pass, &recovery, &pkg).unwrap();
        let bytes = std::fs::read(&pkg).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        let pass_str = String::from_utf8_lossy(pass);
        assert!(
            !s.contains(&*pass_str),
            "backup must not contain the plaintext passphrase"
        );
    }

    #[test]
    fn restore_refuses_path_traversal_in_body() {
        let dest = TempDir::new().unwrap();
        let id = VaultId::new();
        let body = BackupBody {
            vault_id: id.to_string(),
            app_version: "0.1.0".into(),
            schema_version: 1,
            files: vec![BackupFile {
                rel_path: "../escape.txt".into(),
                bytes: vec![1, 2, 3],
                sha256: hex::encode(Sha256::digest([1u8, 2, 3])),
            }],
        };
        let header = BackupHeader {
            format_version: 1,
            vault_id: id.to_string(),
            vault_name: "P".into(),
            created_at: "t".into(),
            passphrase_salt: [0u8; 16],
            argon2_params: argon2_params::Params::minimum_safe(),
            wrapped_bdk_passphrase: "x".into(),
            wrapped_bdk_recovery: "x".into(),
            body_sha256: "x".into(),
            plaintext_body_sha256: "x".into(),
        };
        let err = install_body(dest.path(), &header, &body).unwrap_err();
        assert!(matches!(err, AppError::Vault(_)));
    }
}
