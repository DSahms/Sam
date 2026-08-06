//! Argon2id parameter selection.
//!
//! Directive §10: "Calibrate Argon2id on the target computer while enforcing
//! secure minimum settings."
//!
//! We expose [`Params::minimum_safe`] as the production default (meeting OWASP
//! 2023 minimums: m=19 MiB, t=2, p=1) and [`Params::calibrate`] as a hook for
//! the Phase 9 on-target calibration step. A fast variant exists for tests so
//! the test suite stays quick; it MUST NOT be used in production.

use serde::{Deserialize, Serialize};

/// Argon2id parameters. Stored alongside each vault's wrapped key so a vault
/// created with older parameters can still be unlocked if the default changes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Params {
    /// Memory cost in KiB.
    pub m_kib: u32,
    /// Time cost (iterations).
    pub t: u32,
    /// Parallelism (lanes).
    pub p: u32,
}

impl Params {
    /// OWASP-recommended minimum for Argon2id (2023): 19 MiB, t=2, p=1.
    /// This is the secure floor; production may calibrate higher.
    pub const fn minimum_safe() -> Self {
        Self {
            m_kib: 19 * 1024,
            t: 2,
            p: 1,
        }
    }

    /// Production default. Currently equal to [`minimum_safe`]; Phase 9 will
    /// replace this with a calibrated value per machine.
    pub const fn production_default() -> Self {
        Self::minimum_safe()
    }

    /// Fast parameters for unit tests only. Never use in production paths.
    #[cfg(test)]
    pub const fn test_fast() -> Self {
        Self {
            m_kib: 8 * 1024,
            t: 1,
            p: 1,
        }
    }

    /// Convert to `argon2::Params` (output length fixed at 32 bytes for a KEK).
    pub(crate) fn to_argon2(self) -> argon2::Params {
        // Output length is fixed to 32 bytes (AES-256 key size).
        argon2::Params::new(self.m_kib, self.t, self.p, Some(32))
            .expect("valid Argon2id params")
    }

    /// Calibrate Argon2id parameters to take roughly `target_ms` on this
    /// machine, never below [`minimum_safe`]. Returns a value suitable for
    /// storing with a new vault.
    ///
    /// This is the Phase 9 calibration hook; it is exercised in tests but the
    /// production default remains [`production_default`] until calibration is
    /// wired into the new-vault flow.
    pub fn calibrate(target_ms: u32) -> Self {
        let floor = Self::minimum_safe();
        // Start from the floor and bump time cost while under target. Memory
        // is held at the floor to keep memory pressure predictable; real
        // calibration (Phase 9) will also tune memory.
        let mut best = floor;
        let mut t = floor.t;
        while t < 16 {
            let candidate = Params {
                m_kib: floor.m_kib,
                t,
                p: floor.p,
            };
            if measure_ms(candidate) <= target_ms as u128 {
                best = candidate;
            } else {
                break;
            }
            t += 1;
        }
        best
    }
}

/// Measure how long one Argon2id derivation takes with these params, in ms.
fn measure_ms(p: Params) -> u128 {
    use argon2::password_hash::SaltString;
    let salt = SaltString::from_b64("AAAAAAAAAAAAAAAAAAAAAA").unwrap();
    let a2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        p.to_argon2(),
    );
    let mut out = [0u8; 32];
    let start = std::time::Instant::now();
    let _ = a2.hash_password_into(b"measure", salt.as_str().as_bytes(), &mut out);
    start.elapsed().as_millis().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_safe_meets_owasp_floor() {
        let p = Params::minimum_safe();
        assert!(p.m_kib >= 19 * 1024);
        assert!(p.t >= 2);
        assert!(p.p >= 1);
    }

    #[test]
    fn params_round_trip_serde() {
        let p = Params::minimum_safe();
        let s = serde_json::to_string(&p).unwrap();
        let back: Params = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn calibrate_never_goes_below_floor() {
        let c = Params::calibrate(1);
        let f = Params::minimum_safe();
        assert!(c.m_kib >= f.m_kib);
        assert!(c.t >= f.t);
        assert!(c.p >= f.p);
    }
}
