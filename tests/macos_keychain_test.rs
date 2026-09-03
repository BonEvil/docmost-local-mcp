#[cfg(target_os = "macos")]
mod macos_keychain {
    use std::time::{SystemTime, UNIX_EPOCH};

    use docmost_local_mcp::{storage::keyring_store::KeyringStore, types::StoredCredentials};

    struct KeychainCleanup {
        origin: String,
    }

    impl Drop for KeychainCleanup {
        fn drop(&mut self) {
            let _ = KeyringStore.delete_credentials(&self.origin);
        }
    }

    #[test]
    fn release_build_can_round_trip_credentials_through_macos_keychain() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let origin = format!(
            "https://keychain-test-{}-{nonce}.invalid",
            std::process::id()
        );
        let cleanup = KeychainCleanup {
            origin: origin.clone(),
        };
        let credentials = StoredCredentials {
            origin: Some(origin.clone()),
            email: "keychain-test@example.invalid".to_string(),
            password: "synthetic-keychain-test-value".to_string(),
        };

        KeyringStore
            .write_credentials(&origin, &credentials)
            .expect("the macOS build must persist credentials in Keychain");
        assert_eq!(
            KeyringStore
                .read_credentials(&origin)
                .expect("the macOS build must read credentials from Keychain"),
            Some(credentials)
        );

        KeyringStore
            .delete_credentials(&origin)
            .expect("the macOS build must delete credentials from Keychain");
        assert_eq!(
            KeyringStore
                .read_credentials(&origin)
                .expect("the deleted Keychain entry must be readable as absent"),
            None
        );

        drop(cleanup);
    }
}
