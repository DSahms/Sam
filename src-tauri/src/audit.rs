//! `audit` — append-only audit history.
//!
//! **Phase 1 foundation.** Records security-relevant events (vault access,
//! provider calls, exports, imports, permission use, backup activity) in an
//! append-only table per vault. Audit records never retain deleted private
//! content (directive §19) or full private prompts (directive §13).
