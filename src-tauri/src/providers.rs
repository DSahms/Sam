//! `providers` — replaceable AI provider adapters.
//!
//! **Phase 2.** Stable provider interface plus three adapters: deterministic
//! mock, KoboldCpp-compatible local, Venice-compatible cloud. Provider-specific
//! behavior stays inside adapters; changing providers must never erase
//! identity, conversations, knowledge, or memory (directive §13).
