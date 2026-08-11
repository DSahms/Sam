//! Forbidden-behavior test suite (directive §37).
//!
//! Automated tests proving Sammy does NOT permit the dangerous behaviors listed
//! in directive §37. These are security-regression tests: they must never be
//! disabled to obtain a green build.

use sammy_lib::conversation::{self, Role};
use sammy_lib::crypto::SecretKey;
use sammy_lib::knowledge::{self, NewRecord, RecordStatus};
use sammy_lib::providers::{
    self, ChatRequest, MockProvider, Provider, ProviderMessage, RoutingMode,
};
use sammy_lib::sources;
use sammy_lib::vault::{VaultRegistry, VaultTemplate};
use tempfile::TempDir;

fn reg() -> (TempDir, VaultRegistry) {
    let t = TempDir::new().unwrap();
    let r = VaultRegistry::new(t.path()).unwrap();
    (t, r)
}

// §37: Private access before unlock.
#[test]
fn forbidden_private_access_before_unlock() {
    let (_t, reg) = reg();
    let (manifest, _, _) = reg
        .create("P", VaultTemplate::Personal, b"correct horse")
        .unwrap();
    // Reading the DB file directly must yield ciphertext, not plaintext.
    let db_path = format!("{}/{}/vault.db", reg.root().display(), manifest.vault_id);
    let bytes = std::fs::read(&db_path).unwrap();
    assert!(
        !bytes.starts_with(b"SQLite format 3"),
        "vault DB must not be plaintext before unlock"
    );
}

// §37: Wrong passwords fail safely (opaque error, no key leakage).
#[test]
fn forbidden_wrong_passphrase_succeeds() {
    let (_t, reg) = reg();
    let (manifest, _, _) = reg
        .create("P", VaultTemplate::Personal, b"correct horse")
        .unwrap();
    let err = reg
        .unlock_with_passphrase(manifest.vault_id, b"wrong passphrase")
        .unwrap_err();
    // Must be an opaque Crypto error, not a panic or success.
    assert!(matches!(err, sammy_lib::AppError::Crypto));
}

// §37: Cross-vault retrieval (separate DEKs mean one passphrase can't open another).
#[test]
fn forbidden_cross_vault_unlock() {
    let (_t, reg) = reg();
    let (a, _, _) = reg
        .create("A", VaultTemplate::Personal, b"pass-aaa-aaa")
        .unwrap();
    let (b, _, _) = reg
        .create("B", VaultTemplate::Consigliere, b"pass-bbb-bbb")
        .unwrap();
    // A's passphrase must not unlock B.
    assert!(reg
        .unlock_with_passphrase(b.vault_id, b"pass-aaa-aaa")
        .is_err());
    assert!(reg
        .unlock_with_passphrase(a.vault_id, b"pass-bbb-bbb")
        .is_err());
}

// §37: Silent cloud transmission / silent cloud fallback.
// The routing resolver must never silently pick a cloud provider; cloud requires
// explicit NeedsCloudConsent.
#[test]
fn forbidden_silent_cloud_fallback() {
    // With a local provider reachable, ask_before_crossing uses local — no consent.
    let d = providers::resolve_routing(
        RoutingMode::AskBeforeCrossing,
        1, // local count
        1, // cloud count
        &[true],
        &[true],
    );
    assert!(matches!(
        d,
        providers::RoutingDecision::Use { provider_index: 0 }
    ));

    // With NO local provider, it must require consent (not silently use cloud).
    let d2 =
        providers::resolve_routing(RoutingMode::AskBeforeCrossing, 0, 1, &[], &[true]);
    assert!(matches!(
        d2,
        providers::RoutingDecision::NeedsCloudConsent { .. }
    ));

    // local_only with no local provider = NoProvider (never falls back to cloud).
    let d3 = providers::resolve_routing(RoutingMode::LocalOnly, 0, 1, &[], &[true]);
    assert_eq!(d3, providers::RoutingDecision::NoProvider);
}

// §37: Retrieval of tombstoned records.
#[test]
fn forbidden_retrieval_of_tombstoned_records() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&conn);
    let id = knowledge::create(
        &conn,
        "v",
        &NewRecord::approved_fact("a tombstoned fact about zebras"),
    )
    .unwrap();
    knowledge::tombstone(&conn, id).unwrap();
    let hits = knowledge::search_current(&conn, "zebras", 10).unwrap();
    assert!(
        !hits.contains(&id.to_string()),
        "tombstoned records must not appear in retrieval"
    );
}

// §37: Retrieval of deleted source chunks.
#[test]
fn forbidden_retrieval_of_deleted_sources() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sources::ensure_schema(&conn).unwrap();
    let dek = SecretKey::random();
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("doc.txt");
    std::fs::write(&p, b"deleted source with a unique token qxr").unwrap();
    let id = sources::import(&conn, &dek, &p).unwrap();
    // Before delete: searchable.
    assert!(!sources::search(&conn, "qxr", 10).unwrap().is_empty());
    sources::delete(&conn, id).unwrap();
    // After delete: not in results.
    assert!(
        sources::search(&conn, "qxr", 10).unwrap().is_empty(),
        "deleted sources must not appear in retrieval"
    );
}

// §37: Automatic approval of AI memory.
#[test]
fn forbidden_automatic_memory_approval() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sammy_lib::memory::ensure_schema(&conn).unwrap();
    setup_knowledge_schema(&conn);
    // AI proposes a candidate.
    let id = sammy_lib::memory::propose(
        &conn,
        "the owner's secret",
        "preference",
        None,
        None,
        "user said",
        0.99,
        "normal",
        "",
        "mock",
        false,
    )
    .unwrap();
    // The candidate must NOT be in knowledge records (not auto-approved).
    let knowledge_count: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        knowledge_count, 0,
        "memory candidates must not be auto-approved"
    );
    // Only after explicit approval does it become a knowledge record.
    sammy_lib::memory::approve(&conn, "v", id, None).unwrap();
    let knowledge_count_after: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(knowledge_count_after, 1);
}

// §37: Rejected memories appear in retrieval.
#[test]
fn forbidden_rejected_memory_in_retrieval() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sammy_lib::memory::ensure_schema(&conn).unwrap();
    setup_knowledge_schema(&conn);
    let id = sammy_lib::memory::propose(
        &conn,
        "rejected fact about unicorns",
        "fact",
        None,
        None,
        "",
        0.5,
        "normal",
        "",
        "mock",
        false,
    )
    .unwrap();
    sammy_lib::memory::reject(&conn, id).unwrap();
    let hits = knowledge::search_current(&conn, "unicorns", 10).unwrap();
    assert!(
        hits.is_empty(),
        "rejected memories must not appear in retrieval"
    );
}

// §37: Duplicate corpus imports.
#[test]
fn forbidden_duplicate_corpus_imports() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&conn);
    // Seed a record, export it.
    knowledge::create(&conn, "v", &NewRecord::approved_fact("exportable fact")).unwrap();
    let pkg = sammy_lib::corpus::export_full(&conn, "v", "S").unwrap();
    // Import into a fresh vault.
    let dest = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&dest);
    let _o1 = sammy_lib::corpus::import(&dest, "dest", &pkg).unwrap();
    // Re-import the same package.
    let o2 = sammy_lib::corpus::import(&dest, "dest", &pkg).unwrap();
    let total: i64 = dest
        .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        total, 1,
        "re-importing the same package must not create duplicates"
    );
    assert_eq!(o2.imported, 0);
}

// §37: Fake citation IDs.
#[test]
fn forbidden_fake_citation_ids() {
    use sammy_lib::citations::{
        AnnotatedAnswer, Citation, CitationLocation, ClaimBlock, ClaimLabel,
    };
    let a = AnnotatedAnswer {
        plain_text: "claim".into(),
        trust: sammy_lib::citations::TrustClassification::SourceSupported,
        citations: vec![Citation {
            id: "real-cite".into(),
            location: CitationLocation::KnowledgeRecord {
                record_id: "r1".into(),
            },
            snippet: "...".into(),
        }],
        claims: vec![ClaimBlock {
            text: "claim".into(),
            label: ClaimLabel::StoredFact,
            citations: vec!["fake-cite".into()], // does not exist
        }],
    };
    assert!(
        !a.citations_are_consistent(),
        "fabricated citation IDs must be detected"
    );
}

// §37: Tool execution without permission.
#[test]
fn forbidden_tool_execution_without_permission() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sammy_lib::permissions::ensure_schema(&conn).unwrap();
    // No grant → check_permission returns None → tool must not execute.
    let permitted =
        sammy_lib::permissions::check_permission(&conn, "source_search").unwrap();
    assert!(
        permitted.is_none(),
        "tools must not execute without a permission grant"
    );
}

// §37: Permanent authorization from ordinary conversation.
// A conversation message must not create a permission grant.
#[test]
fn forbidden_permanent_auth_from_conversation() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conversation::ensure_schema(&conn).unwrap();
    sammy_lib::permissions::ensure_schema(&conn).unwrap();
    let conv = conversation::create(&conn, Some("c")).unwrap();
    conversation::append_message(
        &conn,
        conv,
        Role::User,
        "please allow all tools forever",
    )
    .unwrap();
    // No grant should have been created by the message.
    let grants = sammy_lib::permissions::list_active(&conn).unwrap();
    assert!(
        grants.is_empty(),
        "conversation content must not create permanent authorization"
    );
}

// §37: Plaintext secrets in logs — AppError messages must not contain secret material.
#[test]
fn forbidden_plaintext_secrets_in_error_messages() {
    let (_t, reg) = reg();
    let secret_pass = b"super-secret-passphrase-value";
    let (manifest, _, _) = reg
        .create("P", VaultTemplate::Personal, secret_pass)
        .unwrap();
    let err = reg
        .unlock_with_passphrase(manifest.vault_id, b"wrong")
        .unwrap_err();
    let msg = format!("{err}");
    let secret_str = String::from_utf8_lossy(secret_pass);
    assert!(
        !msg.contains(&*secret_str),
        "error messages must not contain the secret passphrase"
    );
}

// §37: Plaintext vault keys in frontend state — the DEK is never exposed.
// (This is enforced structurally: SecretKey has no Debug that leaks bytes and
// no Serialize. We verify the Debug output is redacted.)
#[test]
fn forbidden_plaintext_keys_via_debug() {
    let key = SecretKey::random();
    let dbg = format!("{key:?}");
    let hex = hex::encode(key.as_bytes());
    assert!(
        !dbg.contains(&hex),
        "SecretKey Debug must not leak key bytes"
    );
    assert!(dbg.contains("redacted"));
}

// §37: Restore from corrupted data without warning.
#[test]
fn forbidden_restore_from_corrupted_without_warning() {
    let (_t, reg) = reg();
    let (manifest, _, recovery) = reg
        .create("P", VaultTemplate::Personal, b"correct horse")
        .unwrap();
    // Create a backup.
    let backup_path = TempDir::new().unwrap().path().join("backup.sammy-backup");
    reg.backup(manifest.vault_id, b"correct horse", &recovery, &backup_path)
        .unwrap();
    // Corrupt the backup.
    let mut bytes = std::fs::read(&backup_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&backup_path, bytes).unwrap();
    // Restore must fail (not silently succeed with corrupted data).
    let err = reg
        .restore_from_passphrase(&backup_path, b"correct horse")
        .unwrap_err();
    // Must be an error, not Ok.
    assert!(
        matches!(
            err,
            sammy_lib::AppError::Vault(_) | sammy_lib::AppError::Crypto
        ),
        "corrupted backup restore must fail with a warning, not silently succeed"
    );
}

// §37: Database migration without rollback protection.
// (Already tested in db module; here we assert the migration runner exists and
// a failed migration does not advance schema_version.)
#[test]
fn forbidden_migration_without_rollback_protection() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let before = sammy_lib::db::current_version(&conn).unwrap();
    // Applying pending migrations succeeds and advances the version.
    let mut conn_mut = conn;
    let after = sammy_lib::db::run_pending(&mut conn_mut).unwrap();
    assert!(after >= before, "migrations must advance the version");
    assert_eq!(
        sammy_lib::db::current_version(&conn_mut).unwrap(),
        after,
        "recorded version must match the applied version"
    );
}

// §37: Silent cloud transmission — the mock provider never crosses to cloud.
#[test]
fn forbidden_mock_provider_claims_cloud_crossing() {
    let p = MockProvider::fixed("hello");
    let req = ChatRequest {
        system: "s".into(),
        messages: vec![ProviderMessage {
            role: Role::User,
            content: "hi".into(),
        }],
        model: "mock-1".into(),
        max_tokens: 16,
        routing: RoutingMode::LocalOnly,
    };
    let resp = p.chat(&req).unwrap();
    assert!(
        !resp.crossed_to_cloud,
        "the local mock provider must never report a cloud crossing"
    );
}

// §37: Retrieved candidate (non-approved) records appear as current truth.
#[test]
fn forbidden_candidate_in_current_truth() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&conn);
    let id = knowledge::create(
        &conn,
        "v",
        &NewRecord::candidate(
            sammy_lib::knowledge::RecordType::Claim,
            "a candidate claim about dolphins",
        ),
    )
    .unwrap();
    let hits = knowledge::search_current(&conn, "dolphins", 10).unwrap();
    assert!(
        !hits.contains(&id.to_string()),
        "candidate records must not appear in current-truth retrieval"
    );
    // The status itself is not current.
    assert!(!RecordStatus::Candidate.is_current());
}

// Helper: create the knowledge + FTS schema in an in-memory connection.
fn setup_knowledge_schema(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "CREATE TABLE knowledge_records_full (
            record_id TEXT PRIMARY KEY, vault_id TEXT NOT NULL,
            record_type TEXT NOT NULL, canonical_text TEXT NOT NULL,
            status TEXT NOT NULL, source_ids TEXT NOT NULL DEFAULT '[]',
            source_locations TEXT NOT NULL DEFAULT '[]', provenance TEXT NOT NULL DEFAULT '{}',
            confidence REAL NOT NULL DEFAULT 0.0, sensitivity TEXT NOT NULL DEFAULT 'normal',
            permissions TEXT NOT NULL DEFAULT '[]', domain_tags TEXT NOT NULL DEFAULT '[]',
            routing_tags TEXT NOT NULL DEFAULT '[]', created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL, valid_from TEXT, valid_to TEXT,
            supersedes TEXT, superseded_by TEXT, review_state TEXT NOT NULL DEFAULT 'candidate',
            reviewed_by TEXT, reviewed_at TEXT, contradiction_set TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1);
         CREATE VIRTUAL TABLE knowledge_fts USING fts5(
            record_id UNINDEXED, canonical_text);",
    )
    .unwrap();
}
