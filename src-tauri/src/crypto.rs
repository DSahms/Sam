//! `crypto` — cryptographic primitives for the vault spine (Phase 1).
//!
//! Provides the building blocks used by `vault`, `backup`, and the source
//! store. Nothing here is provider- or vault-aware; this module is pure
//! cryptography.
//!
//! # Layout
//! - [`SecretKey`] — a 32-byte key that zeroizes on drop.
//! - [`random`]] — CSPRNG-backed generation.
//! - [`argon2`] — Argon2id key-encryption-key derivation.
//! - [`aead`] — AES-256-GCM encryption.
//! - [`wrap`] — wrap/unwrap a data-encryption key under a key-encryption key.
//! - [`VaultKeyMaterial`] — the on-disk record storing wrapped keys + params.
//! - [`recovery`] — high-entropy recovery key generation and wrapping.
//!
//! See `docs/KEY_MANAGEMENT.md` for the full wrapping hierarchy.
//!
//! # Secret handling
//! Secret material is held in [`SecretKey`] / `SecretBytes` which zeroize on
//! drop. Errors are opaque ([`CryptoError`]) and never include key bytes.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Version};
use base64::Engine;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod argon2_params;
pub mod recovery;

pub use recovery::{RecoveryCode, RECOVERY_CODE_BYTES};

/// A 32-byte secret key (e.g. a vault data-encryption key or a derived
/// key-encryption key). Zeroed on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey([u8; 32]);

/// Manual `Debug` that does not leak key bytes.
impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(<redacted>)")
    }
}

impl SecretKey {
    /// Wrap an existing 32-byte value. The caller is responsible for how the
    /// bytes were produced.
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    /// Generate a fresh random key using the application CSPRNG.
    pub fn random() -> Self {
        let mut b = [0u8; 32];
        random::fill(&mut b);
        Self(b)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Constant-time comparison. Returns `true` if the keys are equal.
    pub fn ct_eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0).into()
    }
}

/// `PartialEq` implemented with constant-time comparison so naive `==` on
/// secret keys is still side-channel-resistant.
impl PartialEq for SecretKey {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other)
    }
}
impl Eq for SecretKey {}

/// Variable-length secret bytes (e.g. a wrapped key blob). Zeroed on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    pub fn from_vec(v: Vec<u8>) -> Self {
        Self(v)
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Opaque crypto error. Carries no secret material.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("cryptographic verification failed")]
    VerifyFailed,
    #[error("invalid key material length")]
    InvalidLength,
    #[error("invalid salt")]
    InvalidSalt,
    #[error("invalid encoding: {0}")]
    InvalidEncoding(String),
}

// -----------------------------------------------------------------------------
// CSPRNG
// -----------------------------------------------------------------------------

pub mod random {
    use super::*;

    /// The application CSPRNG. We use ChaCha20Rng seeded from the OS RNG,
    /// which is a auditable, deterministic-from-seed DRBG that is widely
    /// accepted for cryptographic use. Each call reseeds.
    ///
    /// (For the very highest assurance one could use `OsRng` directly on every
    /// call; ChaCha20Rng seeded from OsRng is the conventional, faster choice
    /// and is what SQLCipher and similar libraries effectively rely on.)
    pub fn rng() -> ChaCha20Rng {
        ChaCha20Rng::from_entropy()
    }

    /// Fill `out` with cryptographically secure random bytes.
    pub fn fill(out: &mut [u8]) {
        rng().fill_bytes(out);
    }

    /// Generate `n` random bytes.
    pub fn bytes(n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        fill(&mut v);
        v
    }

    /// Generate a random 16-byte salt.
    pub fn salt16() -> [u8; 16] {
        let mut s = [0u8; 16];
        fill(&mut s);
        s
    }
}

// -----------------------------------------------------------------------------
// Argon2id key-encryption-key derivation
// -----------------------------------------------------------------------------

/// Derive a 32-byte key-encryption key (KEK) from a passphrase and salt using
/// Argon2id with the given parameters.
///
/// The passphrase is consumed as a byte slice; callers should zeroize their
/// own copy.
pub fn derive_kek(
    passphrase: &[u8],
    salt: &[u8; 16],
    params: &argon2_params::Params,
) -> Result<SecretKey, CryptoError> {
    let salt_string =
        SaltString::encode_b64(salt).map_err(|_| CryptoError::InvalidSalt)?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params.to_argon2());
    let mut out = [0u8; 32];
    a2.hash_password_into(passphrase, salt_string.as_str().as_bytes(), &mut out)
        .map_err(|_| CryptoError::VerifyFailed)?;
    Ok(SecretKey::from_bytes(out))
}

// -----------------------------------------------------------------------------
// AES-256-GCM
// -----------------------------------------------------------------------------

pub mod aead {
    use super::*;

    /// Nonce length for AES-256-GCM (96 bits).
    pub const NONCE_LEN: usize = 12;
    /// Key length for AES-256-GCM.
    pub const KEY_LEN: usize = 32;
    /// Authentication tag length.
    pub const TAG_LEN: usize = 16;

    /// Encrypt `plaintext` under `key` with a freshly generated random nonce.
    /// Returns `nonce || ciphertext+tag`. The nonce is prepended so the result
    /// is self-describing.
    pub fn encrypt(key: &SecretKey, plaintext: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).expect("32-byte key");
        let mut nonce_bytes = [0u8; NONCE_LEN];
        random::fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher.encrypt(nonce, plaintext).expect("AES-GCM encrypt");
        let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        out
    }

    /// Decrypt a blob produced by [`encrypt`]: `nonce || ciphertext+tag`.
    /// Returns the plaintext in a [`SecretBytes`] that zeroizes on drop.
    pub fn decrypt(key: &SecretKey, blob: &[u8]) -> Result<SecretBytes, CryptoError> {
        if blob.len() < NONCE_LEN + TAG_LEN {
            return Err(CryptoError::InvalidLength);
        }
        let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).expect("32-byte key");
        let pt = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ct)
            .map_err(|_| CryptoError::VerifyFailed)?;
        Ok(SecretBytes::from_vec(pt))
    }
}

// -----------------------------------------------------------------------------
// Key wrapping
// -----------------------------------------------------------------------------

/// Wrap a 32-byte data-encryption key (`dek`) under a key-encryption key
/// (`kek`) using AES-256-GCM. Returns `nonce || ciphertext+tag`.
pub fn wrap_key(kek: &SecretKey, dek: &SecretKey) -> Vec<u8> {
    aead::encrypt(kek, dek.as_bytes())
}

/// Unwrap a blob produced by [`wrap_key`] back into a [`SecretKey`].
/// Fails opaquely on tampering or wrong KEK.
pub fn unwrap_key(kek: &SecretKey, blob: &[u8]) -> Result<SecretKey, CryptoError> {
    let pt = aead::decrypt(kek, blob)?;
    if pt.len() != 32 {
        return Err(CryptoError::InvalidLength);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(pt.as_slice());
    Ok(SecretKey::from_bytes(arr))
}

// -----------------------------------------------------------------------------
// On-disk vault key material
// -----------------------------------------------------------------------------

/// Versioned on-disk record holding everything needed to unwrap the vault
/// data-encryption key with the master passphrase. Stored *outside* the
/// SQLCipher database (which is itself encrypted by the DEK).
///
/// Never contains the plaintext passphrase or the plaintext DEK.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultKeyMaterial {
    /// Format version of this record. Bumped on any incompatible change.
    pub version: u32,
    /// Random 16-byte salt for Argon2id.
    pub salt: [u8; 16],
    /// Argon2id parameters used for KDF.
    pub params: argon2_params::Params,
    /// The DEK wrapped under the passphrase-derived KEK
    /// (`nonce || ciphertext+tag`), base64-encoded.
    pub wrapped_dek: String,
    /// The DEK wrapped under the recovery KEK, base64-encoded.
    pub wrapped_dek_recovery: String,
}

impl VaultKeyMaterial {
    pub const VERSION: u32 = 1;

    /// Create fresh key material for a new vault.
    ///
    /// Generates a random DEK and a random salt, derives the passphrase KEK,
    /// and wraps the DEK under both the passphrase KEK and the recovery KEK.
    /// Returns the material plus the plaintext DEK (to open SQLCipher with)
    /// and the freshly-generated recovery code (to show the owner once).
    pub fn create(
        passphrase: &[u8],
        recovery_code: &RecoveryCode,
        params: &argon2_params::Params,
    ) -> Result<(Self, SecretKey), CryptoError> {
        let dek = SecretKey::random();
        let salt = random::salt16();

        let pass_kek = derive_kek(passphrase, &salt, params)?;
        let rec_kek = recovery::derive_recovery_kek(recovery_code, &salt)?;

        let wrapped_dek = wrap_key(&pass_kek, &dek);
        let wrapped_dek_recovery = wrap_key(&rec_kek, &dek);

        let material = Self {
            version: Self::VERSION,
            salt,
            params: *params,
            wrapped_dek: b64().encode(&wrapped_dek),
            wrapped_dek_recovery: b64().encode(&wrapped_dek_recovery),
        };
        Ok((material, dek))
    }

    /// Unwrap the DEK using the master passphrase. Fails opaquely on a wrong
    /// passphrase or tampered material.
    pub fn unlock_with_passphrase(
        &self,
        passphrase: &[u8],
    ) -> Result<SecretKey, CryptoError> {
        let kek = derive_kek(passphrase, &self.salt, &self.params)?;
        let blob = b64()
            .decode(&self.wrapped_dek)
            .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        unwrap_key(&kek, &blob)
    }

    /// Unwrap the DEK using the recovery code. Fails opaquely otherwise.
    pub fn unlock_with_recovery(
        &self,
        recovery_code: &RecoveryCode,
    ) -> Result<SecretKey, CryptoError> {
        let kek = recovery::derive_recovery_kek(recovery_code, &self.salt)?;
        let blob = b64()
            .decode(&self.wrapped_dek_recovery)
            .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        unwrap_key(&kek, &blob)
    }

    /// Serialize to bytes (JSON) for storage outside the database.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("VaultKeyMaterial serializable")
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(b: &[u8]) -> Result<Self, CryptoError> {
        serde_json::from_slice(b).map_err(|e| CryptoError::InvalidEncoding(e.to_string()))
    }
}

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_key_zeroizes_logical_eq() {
        let a = SecretKey::from_bytes([7u8; 32]);
        let b = SecretKey::from_bytes([7u8; 32]);
        let c = SecretKey::from_bytes([8u8; 32]);
        assert!(a == b);
        assert!(!(a == c));
    }

    #[test]
    fn random_keys_are_unique() {
        let a = SecretKey::random();
        let b = SecretKey::random();
        assert!(!(a == b));
    }

    #[test]
    fn aead_round_trip() {
        let key = SecretKey::random();
        let msg = b"the quick brown fox";
        let ct = aead::encrypt(&key, msg);
        let pt = aead::decrypt(&key, &ct).unwrap();
        assert_eq!(pt.as_slice(), msg);
    }

    #[test]
    fn aead_wrong_key_fails() {
        let k1 = SecretKey::random();
        let k2 = SecretKey::random();
        let ct = aead::encrypt(&k1, b"secret");
        assert!(matches!(
            aead::decrypt(&k2, &ct),
            Err(CryptoError::VerifyFailed)
        ));
    }

    #[test]
    fn aead_tamper_fails() {
        let key = SecretKey::random();
        let mut ct = aead::encrypt(&key, b"secret");
        let last = ct.len() - 1;
        ct[last] ^= 0xff;
        assert!(aead::decrypt(&key, &ct).is_err());
    }

    #[test]
    fn aead_short_blob_fails() {
        let key = SecretKey::random();
        assert!(matches!(
            aead::decrypt(&key, &[0u8; 5]),
            Err(CryptoError::InvalidLength)
        ));
    }

    #[test]
    fn wrap_unwrap_round_trip() {
        let kek = SecretKey::random();
        let dek = SecretKey::random();
        let blob = wrap_key(&kek, &dek);
        let recovered = unwrap_key(&kek, &blob).unwrap();
        assert!(dek == recovered);
    }

    #[test]
    fn kek_is_deterministic_for_same_inputs() {
        let p = b"correct horse battery staple";
        let salt = random::salt16();
        let params = argon2_params::Params::minimum_safe();
        let k1 = derive_kek(p, &salt, &params).unwrap();
        let k2 = derive_kek(p, &salt, &params).unwrap();
        assert!(k1 == k2);
    }

    #[test]
    fn kek_differs_for_different_passphrases() {
        let salt = random::salt16();
        let params = argon2_params::Params::minimum_safe();
        let k1 = derive_kek(b"pass1", &salt, &params).unwrap();
        let k2 = derive_kek(b"pass2", &salt, &params).unwrap();
        assert!(!(k1 == k2));
    }

    #[test]
    fn vault_material_full_round_trip_passphrase() {
        let params = argon2_params::Params::test_fast();
        let recovery = RecoveryCode::generate();
        let (material, dek) =
            VaultKeyMaterial::create(b"master-passphrase", &recovery, &params).unwrap();

        let unlocked = material
            .unlock_with_passphrase(b"master-passphrase")
            .unwrap();
        assert!(dek == unlocked);
    }

    #[test]
    fn vault_material_unlock_wrong_passphrase_fails() {
        let params = argon2_params::Params::test_fast();
        let recovery = RecoveryCode::generate();
        let (material, _) =
            VaultKeyMaterial::create(b"correct", &recovery, &params).unwrap();
        assert!(matches!(
            material.unlock_with_passphrase(b"wrong"),
            Err(CryptoError::VerifyFailed)
        ));
    }

    #[test]
    fn vault_material_recovery_unlock_works() {
        let params = argon2_params::Params::test_fast();
        let recovery = RecoveryCode::generate();
        let (material, dek) =
            VaultKeyMaterial::create(b"master", &recovery, &params).unwrap();
        let unlocked = material.unlock_with_recovery(&recovery).unwrap();
        assert!(dek == unlocked);
    }

    #[test]
    fn vault_material_recovery_wrong_code_fails() {
        let params = argon2_params::Params::test_fast();
        let good = RecoveryCode::generate();
        let (material, _) = VaultKeyMaterial::create(b"master", &good, &params).unwrap();
        let wrong = RecoveryCode::generate();
        assert!(material.unlock_with_recovery(&wrong).is_err());
    }

    #[test]
    fn vault_material_round_trips_through_bytes() {
        let params = argon2_params::Params::test_fast();
        let recovery = RecoveryCode::generate();
        let (material, _) =
            VaultKeyMaterial::create(b"master", &recovery, &params).unwrap();
        let bytes = material.to_bytes();
        let back = VaultKeyMaterial::from_bytes(&bytes).unwrap();
        assert_eq!(material, back);
    }

    #[test]
    fn vault_material_does_not_contain_plaintext_dek() {
        let params = argon2_params::Params::test_fast();
        let recovery = RecoveryCode::generate();
        let (material, dek) =
            VaultKeyMaterial::create(b"master", &recovery, &params).unwrap();
        let json = material.to_bytes();
        let dek_hex = hex::encode(dek.as_bytes());
        let dek_b64 = b64().encode(dek.as_bytes());
        let json_str = String::from_utf8(json).unwrap();
        assert!(
            !json_str.contains(&dek_hex),
            "material must not contain plaintext DEK (hex)"
        );
        assert!(
            !json_str.contains(&dek_b64),
            "material must not contain plaintext DEK (b64)"
        );
    }
}
