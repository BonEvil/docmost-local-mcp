use docmost_local_mcp::{
    storage::state_store::StateStore,
    types::{StoredConfig, StoredCredentials, StoredSession},
};
use tempfile::TempDir;

const ORIGIN: &str = "https://docs.example.com";

#[tokio::test]
async fn persists_config_session_and_encrypted_credentials() {
    unsafe {
        std::env::set_var("DOCMOST_DISABLE_KEYRING", "1");
    }
    let temp_dir = TempDir::new().unwrap();
    let store = StateStore::new(Some(temp_dir.path().to_path_buf())).unwrap();

    store
        .write_config(&StoredConfig {
            base_url: "https://docs.example.com".to_string(),
            email: "jane@example.com".to_string(),
            last_authenticated_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
        .await
        .unwrap();
    store
        .write_session(&StoredSession {
            origin: Some(ORIGIN.to_string()),
            token: "token-value".to_string(),
            expires_at: Some("2026-03-12T01:00:00.000Z".to_string()),
            saved_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
        .await
        .unwrap();
    store
        .write_credentials(&StoredCredentials {
            origin: Some(ORIGIN.to_string()),
            email: "jane@example.com".to_string(),
            password: "super-secret".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(
        store.read_config().await.unwrap(),
        Some(StoredConfig {
            base_url: "https://docs.example.com".to_string(),
            email: "jane@example.com".to_string(),
            last_authenticated_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
    );
    assert_eq!(
        store.read_session(ORIGIN).await.unwrap(),
        Some(StoredSession {
            origin: Some(ORIGIN.to_string()),
            token: "token-value".to_string(),
            expires_at: Some("2026-03-12T01:00:00.000Z".to_string()),
            saved_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
    );
    assert_eq!(
        store.read_credentials(ORIGIN).await.unwrap(),
        Some(StoredCredentials {
            origin: Some(ORIGIN.to_string()),
            email: "jane@example.com".to_string(),
            password: "super-secret".to_string(),
        })
    );
}

#[tokio::test]
async fn clears_saved_session_without_touching_credentials() {
    unsafe {
        std::env::set_var("DOCMOST_DISABLE_KEYRING", "1");
    }
    let temp_dir = TempDir::new().unwrap();
    let store = StateStore::new(Some(temp_dir.path().to_path_buf())).unwrap();

    store
        .write_session(&StoredSession {
            origin: Some(ORIGIN.to_string()),
            token: "token-value".to_string(),
            expires_at: None,
            saved_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
        .await
        .unwrap();
    store
        .write_credentials(&StoredCredentials {
            origin: Some(ORIGIN.to_string()),
            email: "jane@example.com".to_string(),
            password: "super-secret".to_string(),
        })
        .await
        .unwrap();

    store.clear_session().await.unwrap();

    assert_eq!(store.read_session(ORIGIN).await.unwrap(), None);
    assert_eq!(
        store.read_credentials(ORIGIN).await.unwrap(),
        Some(StoredCredentials {
            origin: Some(ORIGIN.to_string()),
            email: "jane@example.com".to_string(),
            password: "super-secret".to_string(),
        })
    );
}

#[tokio::test]
async fn never_returns_session_or_credentials_for_a_different_origin() {
    unsafe {
        std::env::set_var("DOCMOST_DISABLE_KEYRING", "1");
    }
    let temp_dir = TempDir::new().unwrap();
    let store = StateStore::new(Some(temp_dir.path().to_path_buf())).unwrap();

    store
        .write_session(&StoredSession {
            origin: Some(ORIGIN.to_string()),
            token: "origin-a-token".to_string(),
            expires_at: None,
            saved_at: "2026-03-12T00:00:00.000Z".to_string(),
        })
        .await
        .unwrap();
    store
        .write_credentials(&StoredCredentials {
            origin: Some(ORIGIN.to_string()),
            email: "jane@example.com".to_string(),
            password: "not-a-real-secret".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(
        store
            .read_session("https://other.example.com")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .read_credentials("https://other.example.com")
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn legacy_unscoped_state_is_not_reused() {
    unsafe {
        std::env::set_var("DOCMOST_DISABLE_KEYRING", "1");
    }
    let temp_dir = TempDir::new().unwrap();
    let store = StateStore::new(Some(temp_dir.path().to_path_buf())).unwrap();

    tokio::fs::create_dir_all(temp_dir.path()).await.unwrap();
    tokio::fs::write(
        temp_dir.path().join("session.json"),
        r#"{"token":"legacy-token","expiresAt":null,"savedAt":"2026-03-12T00:00:00.000Z"}"#,
    )
    .await
    .unwrap();

    assert_eq!(store.read_session(ORIGIN).await.unwrap(), None);
}
