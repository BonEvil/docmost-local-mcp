//! Opt-in desktop integration check using synthetic credentials at a unique origin.
use std::process::Command;

use docmost_local_mcp::{storage::keyring_store::KeyringStore, types::StoredCredentials};

#[test]
fn native_keyring_child() {
    let Ok(origin) = std::env::var("DOCMOST_KEYRING_TEST_ORIGIN") else {
        return;
    };
    let store = KeyringStore;
    let credentials = StoredCredentials {
        origin: Some(origin.clone()),
        email: "synthetic@example.invalid".into(),
        password: "synthetic-keyring-verification-only".into(),
    };
    match std::env::var("DOCMOST_KEYRING_TEST_ACTION")
        .unwrap()
        .as_str()
    {
        "write" => store.write_credentials(&origin, &credentials).unwrap(),
        "read" => assert_eq!(store.read_credentials(&origin).unwrap(), Some(credentials)),
        "delete" => {
            // Delete only this test's entry, never the application's legacy entry.
            keyring::Entry::new("docmost-local-mcp", &format!("credentials|{origin}"))
                .unwrap()
                .delete_credential()
                .unwrap();
            assert!(store.read_credentials(&origin).unwrap().is_none());
        }
        _ => panic!("unknown test action"),
    }
}

#[test]
#[ignore = "requires an unlocked desktop OS keyring; uses only synthetic credentials"]
fn native_credentials_survive_process_restart() {
    let origin = format!(
        "https://keyring-test-{}-{}.example.invalid",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let run = |action: &str| {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "native_keyring_child", "--nocapture"])
            .env("DOCMOST_KEYRING_TEST_ORIGIN", &origin)
            .env("DOCMOST_KEYRING_TEST_ACTION", action)
            .env_remove("DOCMOST_DISABLE_KEYRING")
            .status()
            .unwrap()
            .success()
    };
    assert!(run("write"));
    let persisted = run("read");
    let cleaned = run("delete");
    assert!(persisted, "credentials did not survive a process restart");
    assert!(cleaned, "test credential cleanup failed");
}
