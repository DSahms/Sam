//! `vault` — independent encrypted vaults.
//!
//! **Phase 1.** Owns vault creation, unlock, lock, inactivity/session lock,
//! and multi-vault isolation. Every vault has a separate SQLCipher database,
//! a separate random data-encryption key, separate source storage, and
//! separate audit history. No cross-vault access is permitted.
