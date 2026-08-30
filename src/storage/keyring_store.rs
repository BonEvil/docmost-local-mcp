use anyhow::{Context, Result, anyhow, bail};

use crate::types::StoredCredentials;

const KEYRING_SERVICE: &str = "docmost-local-mcp";
const LEGACY_KEYRING_USERNAME: &str = "credentials";

#[derive(Debug, Clone, Default)]
pub struct KeyringStore;

impl KeyringStore {
    pub fn read_credentials(&self, origin: &str) -> Result<Option<StoredCredentials>> {
        if keyring_disabled() {
            bail!("Secure OS credential storage is unavailable.");
        }

        let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(origin))
            .context("Failed to initialize keyring entry")?;

        match entry.get_password() {
            Ok(value) => {
                let credentials: StoredCredentials = serde_json::from_str(&value)
                    .context("Failed to parse keyring credentials payload")?;
                if credentials.origin.as_deref() == Some(origin) {
                    Ok(Some(credentials))
                } else {
                    Ok(None)
                }
            }
            Err(error) if is_missing_entry(&error) => Ok(None),
            Err(error) => Err(anyhow!(error)).context("Failed to read credentials from keyring"),
        }
    }

    pub fn write_credentials(&self, origin: &str, credentials: &StoredCredentials) -> Result<()> {
        if keyring_disabled() {
            bail!("Secure OS credential storage is unavailable.");
        }

        let username = keyring_username(origin);
        let entry = keyring::Entry::new(KEYRING_SERVICE, &username)
            .context("Failed to initialize keyring entry")?;
        let value = serde_json::to_string(credentials)
            .context("Failed to encode credentials for keyring")?;

        if let Err(error) = entry.set_password(&value) {
            return Err(anyhow!(error)).context("Failed to write credentials to keyring");
        }
        // A build or platform without a real credential store resolves to an entry-scoped
        // store that reports success and retains nothing. Confirm through an independent
        // handle that the secret is actually retrievable, so a remembered password can
        // never be silently discarded: persistence must either work or fail closed.
        if !self.stored_value_is_retrievable(&username, &value) {
            let _ = entry.delete_credential();
            bail!("Secure OS credential storage is unavailable.");
        }
        Ok(())
    }

    /// Read the just-written secret back through a separate entry handle. Any missing,
    /// unreadable, or mismatched value means the store did not durably retain it.
    fn stored_value_is_retrievable(&self, username: &str, expected: &str) -> bool {
        keyring::Entry::new(KEYRING_SERVICE, username)
            .ok()
            .and_then(|verifier| verifier.get_password().ok())
            .is_some_and(|stored| stored == expected)
    }

    pub fn delete_credentials(&self, origin: &str) -> Result<()> {
        if keyring_disabled() {
            return Ok(());
        }
        for username in [
            keyring_username(origin),
            LEGACY_KEYRING_USERNAME.to_string(),
        ] {
            let entry = keyring::Entry::new(KEYRING_SERVICE, &username)
                .context("Failed to initialize keyring entry for deletion")?;
            match entry.delete_credential() {
                Ok(()) => {}
                Err(error) if is_missing_entry(&error) => {}
                Err(error) => {
                    return Err(anyhow!(error))
                        .context("Failed to delete credentials from keyring");
                }
            }
        }
        Ok(())
    }
}

fn keyring_username(origin: &str) -> String {
    format!("{LEGACY_KEYRING_USERNAME}|{origin}")
}

fn keyring_disabled() -> bool {
    matches!(
        std::env::var("DOCMOST_DISABLE_KEYRING").ok().as_deref(),
        Some("1") | Some("true")
    )
}

fn is_missing_entry(error: &keyring::Error) -> bool {
    matches!(error, keyring::Error::NoEntry)
}

#[cfg(test)]
mod tests {
    use super::{KeyringStore, keyring_username};
    use crate::types::StoredCredentials;

    #[test]
    fn keyring_identity_is_origin_specific_and_never_uses_the_legacy_account() {
        assert_eq!(
            keyring_username("https://docs.example.com"),
            "credentials|https://docs.example.com"
        );
        assert_ne!(
            keyring_username("https://docs.example.com"),
            keyring_username("https://other.example.com")
        );
        assert_ne!(keyring_username("https://docs.example.com"), "credentials");
    }

    #[test]
    fn non_persisting_credential_store_fails_closed_instead_of_discarding_the_password() {
        unsafe { std::env::remove_var("DOCMOST_DISABLE_KEYRING") };
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());

        // The mock store accepts `set_password` but retains nothing outside the entry, so a
        // remembered password would otherwise be reported as saved and silently lost.
        let error = KeyringStore
            .write_credentials(
                "https://docs.example.com",
                &StoredCredentials {
                    origin: Some("https://docs.example.com".to_string()),
                    email: "operator@example.com".to_string(),
                    password: "synthetic-test-value".to_string(),
                },
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Secure OS credential storage is unavailable")
        );
    }

    #[test]
    fn keyring_credential_clear_is_idempotent_for_scoped_and_legacy_entries() {
        unsafe { std::env::remove_var("DOCMOST_DISABLE_KEYRING") };
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());

        let store = KeyringStore;
        store
            .delete_credentials("https://docs.example.com")
            .unwrap();
        store
            .delete_credentials("https://docs.example.com")
            .unwrap();
    }
}
