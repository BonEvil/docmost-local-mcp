use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{
        HeaderValue, Response, StatusCode,
        header::{LOCATION, SET_COOKIE},
    },
    response::IntoResponse,
    routing::post,
};
use docmost_local_mcp::{
    auth::manager::AuthManager,
    docmost_client::DocmostClient,
    network_policy::NetworkPolicy,
    storage::state_store::StateStore,
    types::{LoginInput, StartupConfig, StoredConfig, StoredSession},
};
use tempfile::TempDir;
use tokio::{net::TcpListener, time::sleep};

fn test_policy() -> NetworkPolicy {
    NetworkPolicy {
        connect_timeout: Duration::from_millis(100),
        request_timeout: Duration::from_millis(150),
        max_success_body_bytes: 512,
        max_error_body_bytes: 64,
        max_markdown_bytes: 32,
        max_tool_output_bytes: 128,
        max_structured_content_bytes: 256,
        max_search_bytes: 16,
        max_identifier_bytes: 32,
        max_cursor_bytes: 16,
        max_title_bytes: 16,
        max_description_bytes: 32,
        max_list_limit: 100,
    }
}

async fn listen(app: Router) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((origin, task))
}

async fn seeded_client(
    origin: &str,
    temp: &TempDir,
    policy: NetworkPolicy,
) -> Result<DocmostClient> {
    let store = StateStore::new(Some(temp.path().to_path_buf()), true)?;
    store
        .write_config(&StoredConfig {
            base_url: origin.to_string(),
            email: "fixture@example.test".to_string(),
            last_authenticated_at: "2026-08-27T00:00:00.000Z".to_string(),
        })
        .await?;
    store
        .write_session(&StoredSession {
            origin: Some(origin.to_string()),
            email: Some("fixture@example.test".to_string()),
            token: "synthetic-session-token".to_string(),
            expires_at: None,
            saved_at: "2026-08-27T00:00:00.000Z".to_string(),
        })
        .await?;
    let manager = AuthManager::new_with_network_policy(
        StartupConfig {
            base_url: Some(origin.to_string()),
            allow_insecure_loopback_http: true,
            ..StartupConfig::default()
        },
        Some(temp.path().to_path_buf()),
        policy,
    )?;
    Ok(DocmostClient::new_with_network_policy(manager, policy))
}

#[tokio::test]
async fn stalled_api_and_authentication_requests_hit_the_overall_deadline() -> Result<()> {
    async fn stalled() -> impl IntoResponse {
        sleep(Duration::from_secs(5)).await;
        Json(serde_json::json!({"data": []}))
    }
    let app = Router::new()
        .route("/api/spaces", post(stalled))
        .route("/api/auth/login", post(stalled));
    let (origin, task) = listen(app).await?;
    let policy = test_policy();

    let api_temp = TempDir::new()?;
    let client = seeded_client(&origin, &api_temp, policy).await?;
    let started = Instant::now();
    let api_error = format!("{:#}", client.list_spaces().await.unwrap_err());
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(api_error.to_ascii_lowercase().contains("timed out"));
    assert!(!api_error.contains(&origin));

    let auth_temp = TempDir::new()?;
    let manager = AuthManager::new_with_network_policy(
        StartupConfig {
            base_url: Some(origin.clone()),
            allow_insecure_loopback_http: true,
            ..StartupConfig::default()
        },
        Some(auth_temp.path().to_path_buf()),
        policy,
    )?;
    let started = Instant::now();
    let auth_error = format!(
        "{:#}",
        manager
            .login(LoginInput {
                base_url: origin,
                email: "fixture@example.test".to_string(),
                password: "synthetic-password".to_string(),
                remember_password: false,
            })
            .await
            .unwrap_err()
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(auth_error.to_ascii_lowercase().contains("timed out"));
    assert!(!auth_error.contains("fixture@example.test"));
    assert!(!auth_error.contains("synthetic-password"));

    task.abort();
    Ok(())
}

#[tokio::test]
async fn success_and_error_bodies_have_streaming_hard_caps_and_safe_errors() -> Result<()> {
    async fn oversized_success() -> Response<Body> {
        let chunks = futures::stream::iter([
            Ok::<_, Infallible>(axum::body::Bytes::from(vec![b'x'; 300])),
            Ok::<_, Infallible>(axum::body::Bytes::from(vec![b'x'; 213])),
        ]);
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from_stream(chunks))
            .unwrap()
    }
    async fn safe_error() -> Response<Body> {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("PRIVATE-PAGE-CONTENT"))
            .unwrap()
    }
    async fn oversized_error() -> Response<Body> {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(vec![b'x'; 65]))
            .unwrap()
    }
    let app = Router::new()
        .route("/api/spaces", post(oversized_success))
        .route("/api/users/me", post(safe_error))
        .route("/api/version", post(oversized_error));
    let (origin, task) = listen(app).await?;
    let temp = TempDir::new()?;
    let client = seeded_client(&origin, &temp, test_policy()).await?;

    let success_error = client.list_spaces().await.unwrap_err().to_string();
    assert!(success_error.contains("success response body exceeded the 512-byte limit"));

    let safe_error = client.get_current_user().await.unwrap_err().to_string();
    assert!(safe_error.contains("Response body omitted (20 bytes)"));
    assert!(!safe_error.contains("PRIVATE-PAGE-CONTENT"));

    let oversized_error = client.server_version().await;
    assert_eq!(oversized_error, None);

    task.abort();
    Ok(())
}

#[tokio::test]
async fn authentication_success_body_is_bounded_before_state_is_persisted() -> Result<()> {
    async fn oversized_login() -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(
                SET_COOKIE,
                "authToken=synthetic.header.signature; Path=/; HttpOnly",
            )
            .body(Body::from(vec![b'x'; 65]))
            .unwrap()
    }
    let app = Router::new().route("/api/auth/login", post(oversized_login));
    let (origin, task) = listen(app).await?;
    let temp = TempDir::new()?;
    let manager = AuthManager::new_with_network_policy(
        StartupConfig {
            base_url: Some(origin.clone()),
            allow_insecure_loopback_http: true,
            ..StartupConfig::default()
        },
        Some(temp.path().to_path_buf()),
        test_policy(),
    )?;

    let error = manager
        .login(LoginInput {
            base_url: origin.clone(),
            email: "fixture@example.test".to_string(),
            password: "synthetic-password".to_string(),
            remember_password: false,
        })
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("authentication response body exceeded the 64-byte limit"));
    let store = StateStore::new(Some(temp.path().to_path_buf()), true)?;
    assert!(store.read_session(&origin).await?.is_none());

    task.abort();
    Ok(())
}

#[derive(Clone)]
struct RedirectTargetState {
    hits: Arc<AtomicUsize>,
    authorization_hits: Arc<AtomicUsize>,
}

#[tokio::test]
async fn authenticated_api_redirect_is_returned_without_contacting_the_target() -> Result<()> {
    async fn target(
        State(state): State<RedirectTargetState>,
        request: Request,
    ) -> impl IntoResponse {
        state.hits.fetch_add(1, Ordering::SeqCst);
        if request.headers().contains_key("authorization") {
            state.authorization_hits.fetch_add(1, Ordering::SeqCst);
        }
        Json(serde_json::json!({"data": []}))
    }
    let target_state = RedirectTargetState {
        hits: Arc::new(AtomicUsize::new(0)),
        authorization_hits: Arc::new(AtomicUsize::new(0)),
    };
    let target_app = Router::new()
        .route("/capture", post(target))
        .with_state(target_state.clone());
    let (target_origin, target_task) = listen(target_app).await?;
    let location = format!("{target_origin}/capture");

    async fn redirect(State(location): State<String>) -> impl IntoResponse {
        (
            StatusCode::TEMPORARY_REDIRECT,
            [(LOCATION, HeaderValue::from_str(&location).unwrap())],
        )
    }
    let redirect_app = Router::new()
        .route("/api/spaces", post(redirect))
        .with_state(location);
    let (origin, redirect_task) = listen(redirect_app).await?;
    let temp = TempDir::new()?;
    let client = seeded_client(&origin, &temp, test_policy()).await?;

    let error = client.list_spaces().await.unwrap_err().to_string();
    assert!(error.contains("307 Temporary Redirect"));
    assert_eq!(target_state.hits.load(Ordering::SeqCst), 0);
    assert_eq!(target_state.authorization_hits.load(Ordering::SeqCst), 0);

    redirect_task.abort();
    target_task.abort();
    Ok(())
}

#[tokio::test]
async fn search_list_and_markdown_bounds_accept_boundary_and_reject_one_over() -> Result<()> {
    let hits = Arc::new(AtomicUsize::new(0));
    async fn route(State(hits): State<Arc<AtomicUsize>>) -> impl IntoResponse {
        hits.fetch_add(1, Ordering::SeqCst);
        Json(serde_json::json!({"data": {"items": []}}))
    }
    async fn import(State(hits): State<Arc<AtomicUsize>>) -> impl IntoResponse {
        hits.fetch_add(1, Ordering::SeqCst);
        Json(serde_json::json!({
            "data": {"id": "p", "slugId": "p", "title": "p", "spaceId": "s"}
        }))
    }
    let app = Router::new()
        .route("/api/search", post(route))
        .route("/api/pages/recent", post(route))
        .route("/api/pages/import", post(import))
        .with_state(hits.clone());
    let (origin, task) = listen(app).await?;
    let temp = TempDir::new()?;
    let client = seeded_client(&origin, &temp, test_policy()).await?;

    client.search_docs(&"q".repeat(16), None).await?;
    assert!(client.search_docs(&"q".repeat(17), None).await.is_err());
    client.list_pages("s", Some(100), None).await?;
    assert!(client.list_pages("s", Some(0), None).await.is_err());
    assert!(client.list_pages("s", Some(101), None).await.is_err());
    client.import_markdown_page("s", &"m".repeat(32)).await?;
    assert!(
        client
            .import_markdown_page("s", &"m".repeat(33))
            .await
            .is_err()
    );
    assert_eq!(hits.load(Ordering::SeqCst), 3);

    task.abort();
    Ok(())
}
