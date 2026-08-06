//! Recovery code generation, formatting, and wrapping.
//!
//! Directive §11: during vault creation, generate a high-entropy recovery key,
//! use it to create a separate recovery wrapping key, wrap the vault DEK
//! separately for recovery, display it once, and never store the plaintext.
//!
//! Design:
//! - 20 random bytes (160 bits of entropy) from the CSPRNG.
//! - A SHA-256 checksum byte is appended for typo detection on entry.
//! - Encoded as RFC 4648 base32 (no lowercase ambiguity), upper-case, grouped
//!   into blocks of 4 characters separated by dashes for human transcription:
//!   `XXXX-XXXX-...-XXXX`.
//! - The recovery KEK is derived via HKDF over the raw entropy bytes using the
//!   vault salt as info, producing a 32-byte AES-256 KEK. The salt binds the
//!   recovery key to the vault it was created for.

use crate::crypto::{CryptoError, SecretKey};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Number of raw entropy bytes in a recovery code (160 bits).
pub const RECOVERY_CODE_BYTES: usize = 20;

/// A high-entropy recovery code. The inner bytes are the canonical form used
/// for key derivation. Use [`to_human_string`](Self::to_human_string) and
/// [`parse`](Self::parse) for the human form.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct RecoveryCode([u8; RECOVERY_CODE_BYTES]);

/// Manual `Debug` that does not leak the recovery code bytes.
impl std::fmt::Debug for RecoveryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryCode(<redacted>)")
    }
}

impl RecoveryCode {
    /// Generate a fresh random recovery code.
    pub fn generate() -> Self {
        let mut b = [0u8; RECOVERY_CODE_BYTES];
        crate::crypto::random::fill(&mut b);
        Self(b)
    }

    /// The canonical entropy bytes (for HKDF input).
    pub fn as_bytes(&self) -> &[u8; RECOVERY_CODE_BYTES] {
        &self.0
    }

    /// Format as grouped base32 with a checksum digit, e.g.
    /// `K5QX-7M2J-...-9PWA`.
    pub fn to_human_string(&self) -> String {
        format_human(&self.0)
    }

    /// Parse a human-formatted code (with or without dashes / case / checksum).
    pub fn parse(s: &str) -> Result<Self, CryptoError> {
        let bytes = parse_human(s)?;
        Ok(Self(bytes))
    }
}

/// Derive the 32-byte recovery key-encryption key from a recovery code and the
/// vault's salt, using HKDF-SHA256.
pub fn derive_recovery_kek(
    code: &RecoveryCode,
    salt: &[u8; 16],
) -> Result<SecretKey, CryptoError> {
    let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), code.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"sammy-vault-recovery-kek-v1", &mut okm)
        .map_err(|_| CryptoError::VerifyFailed)?;
    Ok(SecretKey::from_bytes(okm))
}

// -----------------------------------------------------------------------------
// Human format: base32 + checksum, grouped.
// -----------------------------------------------------------------------------

const GROUP_LEN: usize = 4;

/// Compute a single checksum byte: the low byte of SHA-256(entropy), XORed with
/// 0xA5 so an all-zero region is not a valid accidental checksum. Stored as one
/// extra base32 symbol (5 bits) appended after the entropy encoding.
fn checksum_byte(entropy: &[u8]) -> u8 {
    let mut h = Sha256::new();
    h.update(b"sammy-recovery-checksum-v1");
    h.update(entropy);
    let d = h.finalize();
    d[0] ^ 0xA5
}

/// Encode entropy + checksum bitstream into base32, then group.
fn format_human(entropy: &[u8]) -> String {
    // Build a bit buffer: 160 bits entropy + ~5 bits checksum is awkward; we
    // instead append the checksum byte as a full byte and base32-encode all 21
    // bytes (which yields 34 base32 chars; we keep them all and group by 4).
    let mut buf = Vec::with_capacity(entropy.len() + 1);
    buf.extend_from_slice(entropy);
    buf.push(checksum_byte(entropy));

    let b32 = base32_encode(&buf);
    // Group into blocks of GROUP_LEN.
    let mut out = String::with_capacity(b32.len() + b32.len() / GROUP_LEN);
    for (i, c) in b32.chars().enumerate() {
        if i > 0 && i % GROUP_LEN == 0 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

/// Parse a human string back to entropy bytes, verifying the checksum.
fn parse_human(s: &str) -> Result<[u8; RECOVERY_CODE_BYTES], CryptoError> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '-')
        .collect();
    let decoded = base32_decode(&cleaned)?;
    if decoded.len() != RECOVERY_CODE_BYTES + 1 {
        return Err(CryptoError::InvalidEncoding(format!(
            "expected {} entropy+checksum bytes, got {}",
            RECOVERY_CODE_BYTES + 1,
            decoded.len()
        )));
    }
    let (entropy, cksum) = decoded.split_at(RECOVERY_CODE_BYTES);
    let expected = checksum_byte(entropy);
    if cksum[0] != expected {
        return Err(CryptoError::VerifyFailed);
    }
    let mut out = [0u8; RECOVERY_CODE_BYTES];
    out.copy_from_slice(entropy);
    Ok(out)
}

// -----------------------------------------------------------------------------
// Tiny base32 (RFC 4648 alphabet, no padding) to avoid pulling a dependency.
// -----------------------------------------------------------------------------

const B32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    for &b in bytes {
        buffer = (buffer << 8) | b as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1F) as usize;
            out.push(B32_ALPHABET[idx] as char);
        }
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1F) as usize;
        out.push(B32_ALPHABET[idx] as char);
    }
    out
}

fn base32_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    let mut out = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    for ch in s.chars() {
        let upper = ch.to_ascii_uppercase();
        let val = B32_ALPHABET
            .iter()
            .position(|&c| c as char == upper)
            .ok_or_else(|| {
                CryptoError::InvalidEncoding(format!("bad base32 char: {ch}"))
            })?;
        buffer = (buffer << 5) | val as u64;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_unique() {
        let a = RecoveryCode::generate();
        let b = RecoveryCode::generate();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn human_round_trip_preserves_bytes() {
        let code = RecoveryCode::generate();
        let s = code.to_human_string();
        let back = RecoveryCode::parse(&s).unwrap();
        assert_eq!(code.as_bytes(), back.as_bytes());
    }

    #[test]
    fn human_form_is_grouped_and_uppercase() {
        let code = RecoveryCode::generate();
        let s = code.to_human_string();
        for c in s.chars() {
            assert!(
                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-',
                "unexpected char {c}"
            );
        }
        // Groups are length 4 separated by '-', except the final group which
        // may be shorter because 21 bytes do not divide evenly into base32.
        let groups: Vec<&str> = s.split('-').collect();
        for (i, group) in groups.iter().enumerate() {
            let is_last = i == groups.len() - 1;
            let max = GROUP_LEN;
            let min = if is_last { 2 } else { GROUP_LEN };
            assert!(
                (min..=max).contains(&group.len()),
                "group {} has invalid length {}: {s}",
                i,
                group.len()
            );
        }
    }

    #[test]
    fn parse_accepts_lowercase_and_spaces() {
        let code = RecoveryCode::generate();
        let s = code.to_human_string().to_lowercase().replace('-', " ");
        let back = RecoveryCode::parse(&s).unwrap();
        assert_eq!(code.as_bytes(), back.as_bytes());
    }

    #[test]
    fn checksum_detects_a_typo() {
        let code = RecoveryCode::generate();
        let s = code.to_human_string();
        // Flip the first alphabetic character to a different valid base32 char.
        let mut chars: Vec<char> = s.chars().collect();
        let idx = chars.iter().position(|c| c.is_ascii_alphabetic()).unwrap();
        let orig = chars[idx];
        let replacement = if orig == 'A' { 'B' } else { 'A' };
        chars[idx] = replacement;
        let tampered: String = chars.into_iter().collect();
        assert!(RecoveryCode::parse(&tampered).is_err());
    }

    #[test]
    fn derive_recovery_kek_is_deterministic() {
        let code = RecoveryCode::generate();
        let salt = [1u8; 16];
        let k1 = derive_recovery_kek(&code, &salt).unwrap();
        let k2 = derive_recovery_kek(&code, &salt).unwrap();
        assert!(k1 == k2);
    }

    #[test]
    fn derive_recovery_kek_differs_for_different_salts() {
        let code = RecoveryCode::generate();
        let k1 = derive_recovery_kek(&code, &[1u8; 16]).unwrap();
        let k2 = derive_recovery_kek(&code, &[2u8; 16]).unwrap();
        assert!(!(k1 == k2));
    }

    #[test]
    fn base32_round_trip() {
        let cases: &[&[u8]] = &[
            b"",
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0x00; 20],
            &[0xff; 20],
        ];
        for c in cases {
            let enc = base32_encode(c);
            let dec = base32_decode(&enc).unwrap();
            assert_eq!(dec.as_slice(), *c, "round trip failed for {c:?}");
        }
    }
}
