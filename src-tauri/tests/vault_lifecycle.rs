//! Integration test: the full Phase 1 vault lifecycle through the public API.
//! Exercises create -> list -> unlock (passphrase) -> lock -> unlock (recovery),
//! and the key Phase 1 security properties from directive §37.

use sammy_lib::crypto::recovery::RecoveryCode;
use sammy_lib::vault::{VaultRegistry, VaultTemplate};
use tempfile::TempDir;

fn reg() -> (TempDir, VaultRegistry) {
    let t = TempDir::new().unwrap();
    let r = VaultRegistry::new(t.path()).unwrap();
    (t, r)
}

#[test]
fn full_lifecycle_passphrase_and_recovery() {
    let (_t, reg) = reg();
    let (manifest, _dek, recovery) = reg
        .create(
            "Personal",
            VaultTemplate::Personal,
            b"correct horse battery",
        )
        .unwrap();

    // Listed.
    let listed = reg.list().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].vault_id, manifest.vault_id);

    // Unlock with passphrase.
    let v = reg
        .unlock_with_passphrase(manifest.vault_id, b"correct horse battery")
        .unwrap();
    assert_eq!(v.name, "Personal");

    // Recovery code round-trips through its human form.
    let human = recovery.to_human_string();
    let parsed = RecoveryCode::parse(&human).unwrap();
    let v2 = reg
        .unlock_with_recovery(manifest.vault_id, &parsed)
        .unwrap();
    assert_eq!(v2.vault_id, manifest.vault_id);
}

#[test]
fn security_database_not_plaintext_on_disk() {
    let (_t, reg) = reg();
    let (m, _, _) = reg
        .create("P", VaultTemplate::Personal, b"passphrase-xx")
        .unwrap();
    let db = std::fs::read(format!("{}/{}/vault.db", reg.root().display(), m.vault_id))
        .unwrap();
    assert!(!db.starts_with(b"SQLite format 3"));
}

#[test]
fn security_cross_vault_isolation() {
    let (_t, reg) = reg();
    let (a, _, _) = reg
        .create("A", VaultTemplate::Personal, b"pass-aaa-aaa")
        .unwrap();
    let (b, _, _) = reg
        .create("B", VaultTemplate::Consigliere, b"pass-bbb-bbb")
        .unwrap();

    // A's passphrase cannot open B, and vice versa.
    assert!(reg
        .unlock_with_passphrase(a.vault_id, b"pass-bbb-bbb")
        .is_err());
    assert!(reg
        .unlock_with_passphrase(b.vault_id, b"pass-aaa-aaa")
        .is_err());

    // Wrong recovery code cannot open either.
    let wrong = RecoveryCode::generate();
    assert!(reg.unlock_with_recovery(a.vault_id, &wrong).is_err());
    let _ = b; // silence
}

#[test]
fn security_manifest_has_no_plaintext_secrets() {
    let (_t, reg) = reg();
    let pass = b"a-secret-passphrase-value";
    let (m, dek, _) = reg.create("P", VaultTemplate::Personal, pass).unwrap();
    let mf = std::fs::read(format!(
        "{}/{}/vault.json",
        reg.root().display(),
        m.vault_id
    ))
    .unwrap();
    let s = String::from_utf8_lossy(&mf);
    let pass_str = String::from_utf8_lossy(pass);
    assert!(!s.contains(&*pass_str));
    assert!(!s.contains(&hex::encode(dek.as_bytes())));
}
