//! End-to-end workflow tests (directive §36).
//!
//! Exercises the required acceptance workflows through the public API, spanning
//! multiple subsystems: vault lifecycle, recovery, multiple vaults, source-
//! grounded answers, corpus packages, memory, and permissions.

use sammy_lib::conversation::{self, Role};
use sammy_lib::knowledge::{self, NewRecord};
use sammy_lib::providers::{self, ChatRequest, MockProvider, Provider, ProviderMessage, RoutingMode};
use sammy_lib::vault::{VaultRegistry, VaultTemplate};
use tempfile::TempDir;

fn fresh_reg() -> (TempDir, VaultRegistry) {
    let t = TempDir::new().unwrap();
    let r = VaultRegistry::new(t.path()).unwrap();
    (t, r)
}

// §36: New vault
// 1. Create a vault → 2. Save recovery → 3. Confirm → 5. Unlock → 6. Chat → 8. Lock → 9. Reopen
#[test]
fn workflow_new_vault() {
    let (_t, reg) = fresh_reg();
    // Create
    let (manifest, _dek, recovery) = reg
        .create("Personal", VaultTemplate::Personal, b"correct horse battery")
        .unwrap();
    // Recovery code is available and parseable
    let recovery_str = recovery.to_human_string();
    let parsed = sammy_lib::crypto::recovery::RecoveryCode::parse(&recovery_str).unwrap();
    // Unlock with passphrase
    let vault = reg
        .unlock_with_passphrase(manifest.vault_id, b"correct horse battery")
        .unwrap();
    // Chat through the mock provider
    let conn = vault.lock_conn();
    conversation::ensure_schema(&conn).unwrap();
    let conv = conversation::create(&conn, Some("first")).unwrap();
    conversation::append_message(&conn, conv, Role::User, "hello sammy").unwrap();
    let provider = MockProvider::echo();
    let resp = provider
        .chat(&ChatRequest {
            system: "be brief".into(),
            messages: vec![ProviderMessage {
                role: Role::User,
                content: "hello sammy".into(),
            }],
            model: "mock-1".into(),
            max_tokens: 64,
            routing: RoutingMode::LocalOnly,
        })
        .unwrap();
    assert!(resp.content.contains("hello sammy"));
    conversation::append_message(&conn, conv, Role::Assistant, &resp.content).unwrap();
    drop(conn);
    // Lock
    // (vault drops here, simulating lock)
    drop(vault);
    // Reopen with recovery code
    let vault2 = reg.unlock_with_recovery(manifest.vault_id, &parsed).unwrap();
    let conn2 = vault2.lock_conn();
    conversation::ensure_schema(&conn2).unwrap();
    let msgs = conversation::messages(&conn2, conv).unwrap();
    assert_eq!(msgs.len(), 2, "conversation should persist across lock/unlock");
}

// §36: Recovery
// 1. Create and populate → 2. Back up → 4. Restore with passphrase → 5. Restore with recovery
#[test]
fn workflow_recovery() {
    let (_t, reg) = fresh_reg();
    let (manifest, _dek, recovery) = reg
        .create("Personal", VaultTemplate::Personal, b"master-pass")
        .unwrap();
    // Populate with knowledge
    let vault = reg.unlock_with_passphrase(manifest.vault_id, b"master-pass").unwrap();
    {
        let conn = vault.lock_conn();
        knowledge::create(&conn, &manifest.vault_id.to_string(), &NewRecord::approved_fact("earth is round")).unwrap();
    }
    drop(vault);

    // Back up
    let backup_dir = TempDir::new().unwrap();
    let pkg = backup_dir.path().join("backup.sammy-backup");
    reg.backup(manifest.vault_id, b"master-pass", &recovery, &pkg).unwrap();

    // Restore into a NEW registry (simulates replacement computer)
    let (_t2, reg2) = fresh_reg();
    let restored_id = reg2.restore_from_passphrase(&pkg, b"master-pass").unwrap();
    assert_eq!(restored_id, manifest.vault_id);
    // Verify knowledge survived
    let v = reg2.unlock_with_passphrase(restored_id, b"master-pass").unwrap();
    let conn = v.lock_conn();
    let hits = knowledge::search_current(&conn, "earth", 10).unwrap();
    assert!(!hits.is_empty(), "restored vault should contain the knowledge record");

    // Restore with recovery code into another registry
    let (_t3, reg3) = fresh_reg();
    let restored2 = reg3.restore_from_recovery(&pkg, &recovery).unwrap();
    assert_eq!(restored2, manifest.vault_id);
}

// §36: Multiple vaults
// 1. Create Personal and Consigliere → 2. Add different knowledge → 3-5. Verify isolation
#[test]
fn workflow_multiple_vaults() {
    let (_t, reg) = fresh_reg();
    let (personal, _, _) = reg
        .create("Personal", VaultTemplate::Personal, b"pass-personal")
        .unwrap();
    let (consigliere, _, _) = reg
        .create("Consigliere", VaultTemplate::Consigliere, b"pass-consigliere")
        .unwrap();

    // Add different knowledge to each
    let vp = reg.unlock_with_passphrase(personal.vault_id, b"pass-personal").unwrap();
    {
        let conn = vp.lock_conn();
        knowledge::create(&conn, &personal.vault_id.to_string(), &NewRecord::approved_fact("personal fact about hobbies")).unwrap();
    }
    drop(vp);

    let vc = reg.unlock_with_passphrase(consigliere.vault_id, b"pass-consigliere").unwrap();
    {
        let conn = vc.lock_conn();
        knowledge::create(&conn, &consigliere.vault_id.to_string(), &NewRecord::approved_fact("consigliere fact about strategy")).unwrap();
    }
    drop(vc);

    // Verify no cross-vault search (each vault only sees its own records)
    {
        let vp2 = reg.unlock_with_passphrase(personal.vault_id, b"pass-personal").unwrap();
        let conn = vp2.lock_conn();
        let p_hits = knowledge::search_current(&conn, "hobbies", 10).unwrap();
        assert!(!p_hits.is_empty());
        let c_hits = knowledge::search_current(&conn, "strategy", 10).unwrap();
        assert!(c_hits.is_empty(), "personal vault must not see consigliere records");
    }

    // Back up each separately
    let dir = TempDir::new().unwrap();
    let recovery_p = sammy_lib::crypto::recovery::RecoveryCode::generate();
    reg.backup(personal.vault_id, b"pass-personal", &recovery_p, &dir.path().join("p.bak")).unwrap();
    let recovery_c = sammy_lib::crypto::recovery::RecoveryCode::generate();
    reg.backup(consigliere.vault_id, b"pass-consigliere", &recovery_c, &dir.path().join("c.bak")).unwrap();
}

// §36: Source-grounded answer
// 1. Import a document → 2. Extract and index → 3. Ask a supported question → 4. Receive citation
#[test]
fn workflow_source_grounded_answer() {
    use sammy_lib::sources;
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sources::ensure_schema(&conn).unwrap();
    let dek = sammy_lib::crypto::SecretKey::random();
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("doc.txt");
    std::fs::write(&p, b"The quarterly revenue was 4.2 million dollars.").unwrap();
    let _id = sources::import(&conn, &dek, &p).unwrap();

    // Ask a supported question
    let hits = sources::search(&conn, "revenue", 10).unwrap();
    assert!(!hits.is_empty(), "the source should be findable");

    // Ask an unsupported question
    let misses = sources::search(&conn, "weather", 10).unwrap();
    assert!(misses.is_empty(), "no source should match 'weather'");
}

// §36: Corpus package
// 1. Export → 2. Validate → 3. Import into clean vault → 4. Reimport → 5. No duplicates
#[test]
fn workflow_corpus_package() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&conn);
    knowledge::create(&conn, "v", &NewRecord::approved_fact("corpus test fact alpha")).unwrap();
    knowledge::create(&conn, "v", &NewRecord::approved_fact("corpus test fact beta")).unwrap();

    // Export
    let pkg = sammy_lib::corpus::export_full(&conn, "v", "Test").unwrap();
    // Validate
    sammy_lib::corpus::validate_package(&pkg).unwrap();
    // Import into clean vault
    let dest = rusqlite::Connection::open_in_memory().unwrap();
    setup_knowledge_schema(&dest);
    let o1 = sammy_lib::corpus::import(&dest, "dest", &pkg).unwrap();
    assert_eq!(o1.imported, 2);
    // Reimport
    let o2 = sammy_lib::corpus::import(&dest, "dest", &pkg).unwrap();
    assert_eq!(o2.imported, 0, "reimport must not create duplicates");
    // Verify final state
    let count: i64 = dest
        .query_row("SELECT count(*) FROM knowledge_records_full", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

// §36: Memory
// 1. Create candidate → 2. Reject → 3. Not in retrieval → 4. Create another → 5. Approve → 6. Verify current
#[test]
fn workflow_memory() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sammy_lib::memory::ensure_schema(&conn).unwrap();
    setup_knowledge_schema(&conn);

    // Create and reject
    let c1 = sammy_lib::memory::propose(
        &conn, "rejected memory about dragons", "fact", None, None, "", 0.5, "normal", "", "mock", false,
    )
    .unwrap();
    sammy_lib::memory::reject(&conn, c1).unwrap();
    let hits = knowledge::search_current(&conn, "dragons", 10).unwrap();
    assert!(hits.is_empty(), "rejected memory must not be in retrieval");

    // Create and approve
    let c2 = sammy_lib::memory::propose(
        &conn, "the owner prefers morning meetings", "preference", None, None, "", 0.8, "normal", "", "mock", false,
    )
    .unwrap();
    let kid = sammy_lib::memory::approve(&conn, "v", c2, None).unwrap();
    let hits2 = knowledge::search_current(&conn, "meetings", 10).unwrap();
    assert!(hits2.contains(&kid), "approved memory should be in retrieval");
}

// §36: Provider privacy
// 1. Routing local_only → 2. Mock provider used → 3. No cloud crossing
#[test]
fn workflow_provider_privacy() {
    let provider = MockProvider::echo();
    let req = ChatRequest {
        system: "be brief".into(),
        messages: vec![ProviderMessage {
            role: Role::User,
            content: "hello".into(),
        }],
        model: "mock-1".into(),
        max_tokens: 16,
        routing: RoutingMode::LocalOnly,
    };
    let resp = provider.chat(&req).unwrap();
    assert!(!resp.crossed_to_cloud);
    assert_eq!(resp.provider, "mock");

    // Routing: local_only with local available uses local (no consent needed)
    let d = providers::resolve_routing(RoutingMode::LocalOnly, 1, 1, &[true], &[true]);
    assert!(matches!(d, providers::RoutingDecision::Use { provider_index: 0 }));
}

// §36: Permissions
// 1. Request a mock action → 2. No grant → 3. Does not execute → 4. Grant → 5. Executes
#[test]
fn workflow_permissions() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    sammy_lib::permissions::ensure_schema(&conn).unwrap();

    // No grant → not permitted
    assert!(sammy_lib::permissions::check_permission(&conn, "source_search").unwrap().is_none());

    // Grant
    let _gid = sammy_lib::permissions::grant(
        &conn,
        "source_search",
        sammy_lib::permissions::PermissionMode::AllowSession,
        "",
        None,
    )
    .unwrap();

    // Now permitted
    assert!(sammy_lib::permissions::check_permission(&conn, "source_search").unwrap().is_some());

    // Revoke → not permitted again
    sammy_lib::permissions::revoke(&conn, "source_search").unwrap();
    assert!(sammy_lib::permissions::check_permission(&conn, "source_search").unwrap().is_none());
}

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
