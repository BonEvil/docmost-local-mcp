use anyhow::{Context, Result, anyhow};

use crate::types::StoredCredentials;

const KEYRING_SERVICE: &str = "docmost-local-mcp";
const LEGACY_KEYRING_USERNAME: &str = "credentials";

#[derive(Debug, Clone, Default)]
pub struct KeyringStore;

impl KeyringStore {
    pub fn read_credentials(&self, origin: &str) -> Result<Option<StoredCredentials>> {
        if keyring_disabled() {
            return Ok(None);
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
            Err(error) if should_fallback(&error) => Ok(None),
            Err(error) => Err(anyhow!(error)).context("Failed to read credentials from keyring"),
        }
    }

    pub fn write_credentials(&self, origin: &str, credentials: &StoredCredentials) -> Result<bool> {
        if keyring_disabled() {
            return Ok(false);
        }

        let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_username(origin))
            .context("Failed to initialize keyring entry")?;
        let value = serde_json::to_string(credentials)
            .context("Failed to encode credentials for keyring")?;

        match entry.set_password(&value) {
            Ok(()) => Ok(true),
            Err(error) if should_fallback(&error) => Ok(false),
            Err(error) => Err(anyhow!(error)).context("Failed to write credentials to keyring"),
        }
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

fn should_fallback(error: &keyring::Error) -> bool {
    matches!(
        error,
        keyring::Error::PlatformFailure(_)
            | keyring::Error::NoStorageAccess(_)
            | keyring::Error::NoEntry
            | keyring::Error::BadEncoding(_)
    )
}

fn is_missing_entry(error: &keyring::Error) -> bool {
    matches!(error, keyring::Error::NoEntry)
}

#[cfg(test)]
mod tests {
    use super::keyring_username;

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
}
