use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{HeaderValue, Response, StatusCode, header::LOCATION},
    routing::post,
};
use docmost_local_mcp::{
    auth::manager::AuthManager,
    docmost_client::DocmostClient,
    network_policy::NetworkPolicy,
    storage::state_store::StateStore,
    types::{StartupConfig, StoredConfig, StoredSession},
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{net::TcpListener, sync::Notify};

const PAGE_ID: &str = "01999999-1111-7111-8111-111111111111";
const SPACE_ID: &str = "01999999-2222-7222-8222-222222222222";
const COMMENT_ID: &str = "01999999-3333-7333-8333-333333333333";

const MODE_NORMAL: u8 = 0;
const MODE_REDIRECT: u8 = 1;
const MODE_STALL_THEN_DELETE: u8 = 2;
const MODE_OVERSIZE_SUCCESS: u8 = 3;
const MODE_INTERRUPTIBLE: u8 = 4;

#[derive(Clone, Default)]
struct DeleteState {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    statuses: Arc<Mutex<VecDeque<StatusCode>>>,
    mode: Arc<AtomicU8>,
    delete_commits: Arc<AtomicUsize>,
    redirect_target_hits: Arc<AtomicUsize>,
    request_started: Arc<Notify>,
    release_interrupted: Arc<Notify>,
    already_deleted: Arc<AtomicBool>,
}

async fn delete_route(
    State(state): State<DeleteState>,
    OriginalUri(uri): OriginalUri,
    Json(body): Json<Value>,
) -> Response<Body> {
    state
        .requests
        .lock()
        .unwrap()
        .push((uri.path().to_string(), body));
    state.request_started.notify_one();

    match state.mode.load(Ordering::SeqCst) {
        MODE_REDIRECT => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::FOUND;
            response
                .headers_mut()
                .insert(LOCATION, HeaderValue::from_static("/redirect-target"));
            return response;
        }
        MODE_STALL_THEN_DELETE => {
            let commit_state = state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(75)).await;
                commit_state.delete_commits.fetch_add(1, Ordering::SeqCst);
                commit_state.already_deleted.store(true, Ordering::SeqCst);
            });
            tokio::time::sleep(Duration::from_millis(150)).await;
            return Response::new(Body::empty());
        }
        MODE_OVERSIZE_SUCCESS => {
            return Response::new(Body::from(vec![b'x'; 257]));
        }
        MODE_INTERRUPTIBLE => {
            state.release_interrupted.notified().await;
            state.delete_commits.fetch_add(1, Ordering::SeqCst);
            return Response::new(Body::empty());
        }
        MODE_NORMAL => {}
        other => panic!("unexpected mode {other}"),
    }

    if let Some(status) = state.statuses.lock().unwrap().pop_front() {
        let mut response = Response::new(Body::from("synthetic hostile details"));
        *response.status_mut() = status;
        return response;
    }
    if state.already_deleted.swap(true, Ordering::SeqCst) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_FOUND;
        return response;
    }
    state.delete_commits.fetch_add(1, Ordering::SeqCst);
    Response::new(Body::empty())
}

async fn redirect_target(State(state): State<DeleteState>) -> Response<Body> {
    state.redirect_target_hits.fetch_add(1, Ordering::SeqCst);
    Response::new(Body::empty())
}

async fn spawn(
    temp: &TempDir,
    network_policy: NetworkPolicy,
) -> Result<(DocmostClient, DeleteState)> {
    unsafe { std::env::set_var("DOCMOST_DISABLE_KEYRING", "1") };
    let state = DeleteState::default();
    let app = Router::new()
        .route("/api/pages/delete", post(delete_route))
        .route("/api/spaces/delete", post(delete_route))
        .route("/api/comments/delete", post(delete_route))
        .route("/redirect-target", post(redirect_target))
        .with_state(state.clone());
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let store = StateStore::new(Some(temp.path().to_path_buf()), true)?;
    store
        .write_config(&StoredConfig {
            base_url: base_url.clone(),
            email: "delete-test@example.invalid".to_string(),
            last_authenticated_at: "2026-09-01T00:00:00.000Z".to_string(),
        })
        .await?;
    store
        .write_session(&StoredSession {
            origin: Some(base_url.clone()),
            email: Some("delete-test@example.invalid".to_string()),
            token: "synthetic-delete-token".to_string(),
            expires_at: None,
            saved_at: "2026-09-01T00:00:00.000Z".to_string(),
        })
        .await?;
    let auth = AuthManager::new(
        StartupConfig {
            base_url: Some(base_url),
            allow_insecure_loopback_http: true,
            allow_insecure_credential_file: true,
            ..StartupConfig::default()
        },
        Some(temp.path().to_path_buf()),
    )?;
    Ok((
        DocmostClient::new_with_network_policy(auth, network_policy),
        state,
    ))
}

#[tokio::test]
async fn delete_contracts_use_exact_v0950_endpoints_and_payloads() -> Result<()> {
    for (kind, id, expected_path, expected_body) in [
        (
            "page",
            PAGE_ID,
            "/api/pages/delete",
            json!({"pageId": PAGE_ID, "permanentlyDelete": false}),
        ),
        (
            "space",
            SPACE_ID,
            "/api/spaces/delete",
            json!({"spaceId": SPACE_ID}),
        ),
        (
            "comment",
            COMMENT_ID,
            "/api/comments/delete",
            json!({"commentId": COMMENT_ID}),
        ),
    ] {
        let temp = TempDir::new()?;
        let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
        match kind {
            "page" => client.delete_page(id).await?,
            "space" => client.delete_space(id).await?,
            "comment" => client.delete_comment(id).await?,
            _ => unreachable!(),
        }
        assert_eq!(
            state.requests.lock().unwrap().as_slice(),
            &[(expected_path.to_string(), expected_body)]
        );
        assert_eq!(state.delete_commits.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn malformed_targets_fail_before_dispatch() -> Result<()> {
    let temp = TempDir::new()?;
    let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
    for result in [
        client.delete_page("slug-not-stable").await,
        client.delete_space("").await,
        client
            .delete_comment("00000000-0000-0000-0000-00000000000g")
            .await,
    ] {
        assert!(result.is_err());
    }
    assert!(state.requests.lock().unwrap().is_empty());
    Ok(())
}

#[tokio::test]
async fn absent_unauthorized_conflict_and_server_errors_never_retry() -> Result<()> {
    for status in [
        StatusCode::NOT_FOUND,
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::CONFLICT,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        let temp = TempDir::new()?;
        let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
        state.statuses.lock().unwrap().push_back(status);
        let error = client
            .delete_comment(COMMENT_ID)
            .await
            .expect_err("hostile status must fail");
        assert!(error.to_string().contains("not confirmed"));
        assert_eq!(state.requests.lock().unwrap().len(), 1, "status={status}");
        assert_eq!(state.delete_commits.load(Ordering::SeqCst), 0);
    }
    Ok(())
}

#[tokio::test]
async fn duplicate_explicit_invocation_deletes_once_then_fails_absent() -> Result<()> {
    let temp = TempDir::new()?;
    let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
    client.delete_comment(COMMENT_ID).await?;
    let error = client
        .delete_comment(COMMENT_ID)
        .await
        .expect_err("already absent target must fail");
    assert!(error.to_string().contains("not confirmed"));
    assert_eq!(state.requests.lock().unwrap().len(), 2);
    assert_eq!(state.delete_commits.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn redirect_is_not_followed() -> Result<()> {
    let temp = TempDir::new()?;
    let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
    state.mode.store(MODE_REDIRECT, Ordering::SeqCst);
    client
        .delete_space(SPACE_ID)
        .await
        .expect_err("redirect must fail closed");
    assert_eq!(state.requests.lock().unwrap().len(), 1);
    assert_eq!(state.redirect_target_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.delete_commits.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn timeout_after_dispatch_is_ambiguous_but_never_retried() -> Result<()> {
    let temp = TempDir::new()?;
    let policy = NetworkPolicy {
        request_timeout: Duration::from_millis(25),
        ..NetworkPolicy::default()
    };
    let (client, state) = spawn(&temp, policy).await?;
    state.mode.store(MODE_STALL_THEN_DELETE, Ordering::SeqCst);
    let error = client
        .delete_page(PAGE_ID)
        .await
        .expect_err("stall must time out");
    assert!(error.to_string().contains("not confirmed"));
    tokio::time::sleep(Duration::from_millis(175)).await;
    assert_eq!(state.requests.lock().unwrap().len(), 1);
    assert_eq!(state.delete_commits.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn oversized_success_response_fails_after_one_dispatch() -> Result<()> {
    let temp = TempDir::new()?;
    let policy = NetworkPolicy {
        max_success_body_bytes: 256,
        ..NetworkPolicy::default()
    };
    let (client, state) = spawn(&temp, policy).await?;
    state.mode.store(MODE_OVERSIZE_SUCCESS, Ordering::SeqCst);
    let error = client
        .delete_space(SPACE_ID)
        .await
        .expect_err("oversized success body must fail");
    assert!(error.to_string().contains("not confirmed"));
    assert_eq!(state.requests.lock().unwrap().len(), 1);
    assert_eq!(state.delete_commits.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn interrupted_call_is_not_replayed() -> Result<()> {
    let temp = TempDir::new()?;
    let (client, state) = spawn(&temp, NetworkPolicy::default()).await?;
    state.mode.store(MODE_INTERRUPTIBLE, Ordering::SeqCst);
    let task = tokio::spawn(async move { client.delete_comment(COMMENT_ID).await });
    state.request_started.notified().await;
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert_eq!(state.requests.lock().unwrap().len(), 1);
    assert_eq!(state.delete_commits.load(Ordering::SeqCst), 0);
    Ok(())
}
