//! Inactivity and session lock handling (Phase 1).
//!
//! Directive §10: default inactivity lock is 15 minutes, with options for 5,
//! 15, 30, 60 minutes, or manual-only. The vault always locks on app exit,
//! Windows session lock, and explicit Lock Vault.
//!
//! This module provides:
//! - [`LockPolicy`] — the configured inactivity duration or manual-only.
//! - [`InactivityWatchdog`] — tracks last-activity time and decides whether the
//!   vault should be locked now.
//!
//! The watchdog is deliberately decoupled from the timer source (a Tauri
//! periodic task or the OS session-lock signal) so it is fully unit-testable.
//! The actual lock call goes through [`AppState::lock`](crate::app::AppState::lock),
//! which drops the unlocked vault session and clears sensitive state.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The inactivity-lock policy. Values come directly from directive §10.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockPolicy {
    /// Lock after 5 minutes of inactivity.
    Minutes5,
    /// Lock after 15 minutes of inactivity (the default).
    Minutes15,
    /// Lock after 30 minutes of inactivity.
    Minutes30,
    /// Lock after 60 minutes of inactivity.
    Minutes60,
    /// Never auto-lock on inactivity; lock only on explicit action, app exit,
    /// or OS session lock.
    ManualOnly,
}

impl LockPolicy {
    /// Directive §10 default.
    pub const DEFAULT: LockPolicy = LockPolicy::Minutes15;

    /// The inactivity duration, or `None` for manual-only.
    pub fn timeout(self) -> Option<Duration> {
        match self {
            LockPolicy::Minutes5 => Some(Duration::from_secs(5 * 60)),
            LockPolicy::Minutes15 => Some(Duration::from_secs(15 * 60)),
            LockPolicy::Minutes30 => Some(Duration::from_secs(30 * 60)),
            LockPolicy::Minutes60 => Some(Duration::from_secs(60 * 60)),
            LockPolicy::ManualOnly => None,
        }
    }
}

impl Default for LockPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Tracks the last time the user was active and decides whether the vault
/// should be locked due to inactivity.
///
/// Construct with a policy and a clock; call [`touch`] on user activity and
/// [`should_lock`] from a periodic ticker. The clock is abstracted via
/// [`Clock`] so tests can advance time deterministically.
pub struct InactivityWatchdog {
    policy: LockPolicy,
    last_activity: Instant,
}

impl InactivityWatchdog {
    pub fn new(policy: LockPolicy, now: Instant) -> Self {
        Self {
            policy,
            last_activity: now,
        }
    }

    /// Record user activity at time `now`.
    pub fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Returns `true` if the inactivity threshold has elapsed since the last
    /// activity. Always `false` for [`LockPolicy::ManualOnly`].
    pub fn should_lock(&self, now: Instant) -> bool {
        match self.policy.timeout() {
            None => false,
            Some(timeout) => now.duration_since(self.last_activity) >= timeout,
        }
    }

    pub fn policy(&self) -> LockPolicy {
        self.policy
    }
}

/// A minimal clock trait for testability. The production wiring uses
/// `Instant::now()`; tests use a fake clock.
pub trait Clock {
    fn now(&self) -> Instant;
}

/// Real wall-clock implementation.
#[derive(Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_15_minutes() {
        assert_eq!(LockPolicy::default(), LockPolicy::Minutes15);
    }

    #[test]
    fn manual_only_never_locks_on_inactivity() {
        let t0 = Instant::now();
        let mut wd = InactivityWatchdog::new(LockPolicy::ManualOnly, t0);
        // A very long time later.
        let much_later = t0 + Duration::from_secs(10 * 365 * 24 * 3600);
        wd.touch(t0);
        assert!(!wd.should_lock(much_later));
    }

    #[test]
    fn five_minute_policy_locks_after_threshold() {
        let t0 = Instant::now();
        let mut wd = InactivityWatchdog::new(LockPolicy::Minutes5, t0);
        wd.touch(t0);
        // 4:59 — not yet.
        assert!(!wd.should_lock(t0 + Duration::from_secs(5 * 60 - 1)));
        // 5:00 — lock.
        assert!(wd.should_lock(t0 + Duration::from_secs(5 * 60)));
    }

    #[test]
    fn touch_resets_the_timer() {
        let t0 = Instant::now();
        let mut wd = InactivityWatchdog::new(LockPolicy::Minutes15, t0);
        wd.touch(t0);
        // Activity at t0 + 10 min resets the window.
        let t1 = t0 + Duration::from_secs(10 * 60);
        wd.touch(t1);
        // 10 min after t1 (20 min after t0) — still under the 15-min window
        // measured from t1.
        assert!(!wd.should_lock(t1 + Duration::from_secs(10 * 60)));
        // 16 min after t1 — lock.
        assert!(wd.should_lock(t1 + Duration::from_secs(16 * 60)));
    }

    #[test]
    fn all_policies_have_expected_timeouts() {
        assert_eq!(
            LockPolicy::Minutes5.timeout(),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            LockPolicy::Minutes15.timeout(),
            Some(Duration::from_secs(900))
        );
        assert_eq!(
            LockPolicy::Minutes30.timeout(),
            Some(Duration::from_secs(1800))
        );
        assert_eq!(
            LockPolicy::Minutes60.timeout(),
            Some(Duration::from_secs(3600))
        );
        assert_eq!(LockPolicy::ManualOnly.timeout(), None);
    }

    #[test]
    fn policy_round_trips_serde() {
        for p in [
            LockPolicy::Minutes5,
            LockPolicy::Minutes15,
            LockPolicy::Minutes30,
            LockPolicy::Minutes60,
            LockPolicy::ManualOnly,
        ] {
            let s = serde_json::to_string(&p).unwrap();
            let back: LockPolicy = serde_json::from_str(&s).unwrap();
            assert_eq!(p, back);
        }
    }
}
