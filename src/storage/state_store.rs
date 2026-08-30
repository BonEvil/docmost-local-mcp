use std::path::{Path, PathBuf};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use tokio::fs;

#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    storage::keyring_store::KeyringStore,
    types::{StoredConfig, StoredCredentials, StoredSession},
};

const DEFAULT_DIRNAME: &str = ".docmost-local-mcp";

#[derive(Debug, Clone)]
pub struct StateStore {
    pub base_dir: PathBuf,
    config_path: PathBuf,
    session_path: PathBuf,
    credentials_path: PathBuf,
    key_path: PathBuf,
    allow_insecure_credential_file: bool,
    keyring: KeyringStore,
    #[cfg(test)]
    fail_next_session_write: Arc<AtomicBool>,
}

#[derive(Debug, Serialize, serde::Deserialize)]
struct EncryptedPayload {
    iv: String,
    tag: String,
    ciphertext: String,
}

impl StateStore {
    pub fn new(base_dir: Option<PathBuf>, allow_insecure_credential_file: bool) -> Result<Self> {
        let base_dir = match base_dir {
            Some(base_dir) => base_dir,
            None => dirs::home_dir()
                .context("Unable to determine the home directory")?
                .join(DEFAULT_DIRNAME),
        };

        Ok(Self {
            config_path: base_dir.join("config.json"),
            session_path: base_dir.join("session.json"),
            credentials_path: base_dir.join("credentials.enc.json"),
            key_path: base_dir.join("credentials.key"),
            allow_insecure_credential_file,
            base_dir,
            keyring: KeyringStore,
            #[cfg(test)]
            fail_next_session_write: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn read_config(&self) -> Result<Option<StoredConfig>> {
        self.read_json_file(&self.config_path).await
    }

    pub async fn write_config(&self, config: &StoredConfig) -> Result<()> {
        self.write_json_file(&self.config_path, config).await
    }

    pub async fn read_session(&self, origin: &str) -> Result<Option<StoredSession>> {
        let session = self
            .read_json_file::<StoredSession>(&self.origin_path("session", origin, "json"))
            .await?;
        Ok(session.filter(|session| session.origin.as_deref() == Some(origin)))
    }

    pub async fn write_session(&self, session: &StoredSession) -> Result<()> {
        #[cfg(test)]
        if self.fail_next_session_write.swap(false, Ordering::SeqCst) {
            bail!("Injected session persistence failure.");
        }
        if session.origin.is_none() {
            bail!("Refusing to persist a session without a canonical origin.");
        }
        let origin = session.origin.as_deref().expect("checked above");
        self.write_json_file(&self.origin_path("session", origin, "json"), session)
            .await
    }

    #[cfg(test)]
    pub(crate) fn fail_next_session_write(&self) {
        self.fail_next_session_write.store(true, Ordering::SeqCst);
    }

    pub async fn clear_session(&self, origin: &str) -> Result<()> {
        remove_file_if_exists(&self.origin_path("session", origin, "json")).await
    }

    pub async fn read_credentials(&self, origin: &str) -> Result<Option<StoredCredentials>> {
        match self.keyring.read_credentials(origin) {
            Ok(Some(credentials)) => return Ok(Some(credentials)),
            Ok(None) => {}
            Err(error) if !self.allow_insecure_credential_file => return Err(error),
            Err(_) => {}
        }

        if !self.allow_insecure_credential_file {
            return Ok(None);
        }

        let Some(payload) = self
            .read_json_file::<EncryptedPayload>(&self.origin_path(
                "credentials",
                origin,
                "enc.json",
            ))
            .await?
        else {
            return Ok(None);
        };

        let key = self.get_or_create_encryption_key(origin).await?;
        let plaintext = decrypt_string(&payload, &key)?;
        let credentials: StoredCredentials =
            serde_json::from_str(&plaintext).context("Failed to parse decrypted credentials")?;
        Ok((credentials.origin.as_deref() == Some(origin)).then_some(credentials))
    }

    pub async fn write_credentials(&self, credentials: &StoredCredentials) -> Result<()> {
        let Some(origin) = credentials.origin.as_deref() else {
            bail!("Refusing to persist credentials without a canonical origin.");
        };
        match self.keyring.write_credentials(origin, credentials) {
            Ok(()) => return Ok(()),
            Err(error) if !self.allow_insecure_credential_file => return Err(error),
            Err(_) => {}
        }

        let key = self.get_or_create_encryption_key(origin).await?;
        let payload = encrypt_string(&serde_json::to_string(credentials)?, &key)?;
        self.write_json_file(
            &self.origin_path("credentials", origin, "enc.json"),
            &payload,
        )
        .await
    }

    /// Remove every remembered credential representation for one canonical origin without
    /// touching its active config or session. Session-only login uses this before committing
    /// the new identity so a later expiry or 401 cannot revive an older remembered account.
    pub async fn clear_credentials(&self, origin: &str) -> Result<()> {
        // Fail before changing filesystem state if the secure-store deletion fails. A caller can
        // then safely leave the existing config/session unchanged and report the failed login.
        self.keyring.delete_credentials(origin)?;
        for path in [
            self.origin_path("credentials", origin, "enc.json"),
            self.origin_path("credentials", origin, "key"),
        ] {
            remove_file_if_exists(&path).await?;
        }
        if self.should_remove_legacy_credentials(origin).await {
            remove_file_if_exists(&self.credentials_path).await?;
            remove_file_if_exists(&self.key_path).await?;
        }
        Ok(())
    }

    pub async fn forget_origin(&self, origin: &str) -> Result<()> {
        self.clear_credentials(origin).await?;
        remove_file_if_exists(&self.origin_path("session", origin, "json")).await?;
        if self.should_remove_legacy_session(origin).await {
            remove_file_if_exists(&self.session_path).await?;
        }
        if self
            .read_config()
            .await?
            .is_some_and(|config| config.base_url == origin)
        {
            remove_file_if_exists(&self.config_path).await?;
        }
        Ok(())
    }

    async fn should_remove_legacy_session(&self, origin: &str) -> bool {
        match self
            .read_json_file::<StoredSession>(&self.session_path)
            .await
        {
            Ok(None) => false,
            Ok(Some(session)) => session
                .origin
                .as_deref()
                .is_none_or(|value| value == origin),
            Err(_) => true,
        }
    }

    async fn should_remove_legacy_credentials(&self, origin: &str) -> bool {
        let payload = self
            .read_json_file::<EncryptedPayload>(&self.credentials_path)
            .await;
        let key = fs::read_to_string(&self.key_path).await;
        match (payload, key) {
            (Ok(None), Err(error)) if error.kind() == std::io::ErrorKind::NotFound => false,
            (Ok(Some(payload)), Ok(key)) => STANDARD
                .decode(key.trim())
                .ok()
                .and_then(|key| decrypt_string(&payload, &key).ok())
                .and_then(|plaintext| serde_json::from_str::<StoredCredentials>(&plaintext).ok())
                .is_none_or(|credentials| {
                    credentials
                        .origin
                        .as_deref()
                        .is_none_or(|value| value == origin)
                }),
            _ => true,
        }
    }

    fn origin_path(&self, kind: &str, origin: &str, extension: &str) -> PathBuf {
        let identity = format!("{:x}", Sha256::digest(origin.as_bytes()));
        self.base_dir.join(format!("{kind}-{identity}.{extension}"))
    }

    async fn ensure_base_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.base_dir)
            .await
            .with_context(|| format!("Failed to create {}", self.base_dir.display()))?;
        set_mode(&self.base_dir, 0o700).await
    }

    async fn read_json_file<T>(&self, file_path: &Path) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        match fs::read_to_string(file_path).await {
            Ok(contents) => Ok(Some(
                serde_json::from_str(&contents)
                    .with_context(|| format!("Failed to parse {}", file_path.display()))?,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("Failed to read {}", file_path.display()))
            }
        }
    }

    async fn write_json_file<T>(&self, file_path: &Path, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.ensure_base_dir().await?;

        let temp_path = file_path.with_extension(format!(
            "{}.tmp",
            file_path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        ));
        let contents = format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .with_context(|| format!("Failed to serialize {}", file_path.display()))?
        );

        fs::write(&temp_path, contents)
            .await
            .with_context(|| format!("Failed to write {}", temp_path.display()))?;
        set_mode(&temp_path, 0o600).await?;
        fs::rename(&temp_path, file_path)
            .await
            .with_context(|| format!("Failed to move {} into place", temp_path.display()))?;
        set_mode(file_path, 0o600).await
    }

    async fn get_or_create_encryption_key(&self, origin: &str) -> Result<Vec<u8>> {
        self.ensure_base_dir().await?;

        // This weaker key file exists only after the operator's explicit acknowledgement.
        if !self.allow_insecure_credential_file {
            bail!("Insecure credential-file fallback is not enabled.");
        }
        let key_path = self.origin_path("credentials", origin, "key");
        match fs::read_to_string(&key_path).await {
            Ok(value) => STANDARD
                .decode(value.trim())
                .context("Failed to decode stored encryption key"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut key = vec![0u8; 32];
                OsRng.fill_bytes(&mut key);
                fs::write(&key_path, STANDARD.encode(&key))
                    .await
                    .with_context(|| format!("Failed to write {}", key_path.display()))?;
                set_mode(&key_path, 0o600).await?;
                Ok(key)
            }
            Err(error) => {
                Err(error).with_context(|| format!("Failed to read {}", key_path.display()))
            }
        }
    }
}

async fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Failed to remove {}", path.display())),
    }
}

fn encrypt_string(plaintext: &str, key: &[u8]) -> Result<EncryptedPayload> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| anyhow::anyhow!("Failed to initialize AES-256-GCM"))?;
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);
    let ciphertext_with_tag = cipher
        .encrypt(Nonce::from_slice(&iv), plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("Failed to encrypt credentials"))?;
    let split_index = ciphertext_with_tag
        .len()
        .checked_sub(16)
        .context("Encrypted payload was shorter than the GCM tag")?;

    let (ciphertext, tag) = ciphertext_with_tag.split_at(split_index);

    Ok(EncryptedPayload {
        iv: STANDARD.encode(iv),
        tag: STANDARD.encode(tag),
        ciphertext: STANDARD.encode(ciphertext),
    })
}

fn decrypt_string(payload: &EncryptedPayload, key: &[u8]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| anyhow::anyhow!("Failed to initialize AES-256-GCM"))?;
    let iv = STANDARD
        .decode(&payload.iv)
        .context("Failed to decode IV")?;
    let tag = STANDARD
        .decode(&payload.tag)
        .context("Failed to decode tag")?;
    let ciphertext = STANDARD
        .decode(&payload.ciphertext)
        .context("Failed to decode ciphertext")?;

    let mut combined = ciphertext;
    combined.extend_from_slice(&tag);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&iv), combined.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to decrypt credentials"))?;

    String::from_utf8(plaintext).context("Decrypted credentials were not valid UTF-8")
}

#[cfg(unix)]
async fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions)
        .await
        .with_context(|| format!("Failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
async fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{StateStore, encrypt_string};

    #[tokio::test]
    async fn legacy_unscoped_encrypted_credentials_are_not_reused() {
        let temp_dir = TempDir::new().unwrap();
        let store = StateStore::new(Some(temp_dir.path().to_path_buf()), true).unwrap();
        let key = store
            .get_or_create_encryption_key("https://docs.example.com")
            .await
            .unwrap();
        let payload = encrypt_string(
            r#"{"email":"legacy@example.com","password":"not-a-real-secret"}"#,
            &key,
        )
        .unwrap();
        store
            .write_json_file(&store.credentials_path, &payload)
            .await
            .unwrap();

        assert_eq!(
            store
                .read_credentials("https://docs.example.com")
                .await
                .unwrap(),
            None
        );
    }
}
