use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use docmost_local_mcp::{
    auth::manager::AuthManager,
    docmost_client::DocmostClient,
    media::{self, AttachmentInput, InsertMediaInput, MediaKind, UploadAttachmentInput},
    storage::state_store::StateStore,
    types::{StartupConfig, StoredConfig, StoredSession},
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU16, AtomicUsize, Ordering},
};
use tempfile::TempDir;

const PAGE: &str = "11111111-1111-4111-8111-111111111111";
const FILE: &str = "22222222-2222-4222-8222-222222222222";
fn row() -> Value {
    json!({"id":FILE,"pageId":PAGE,"fileName":"sample.png","mimeType":"image/png",
        "fileSize":3,"filePath":"PRIVATE_STORAGE_CANARY","creatorId":"PRIVATE_CREATOR_CANARY"})
}
fn document() -> Value {
    json!({"type":"doc","content":[
        {"type":"paragraph","content":[{"type":"text","text":"Keep me"}]},
        {"type":"image","attrs":{"attachmentId":FILE,"src":"/api/files/sample"}},
        {"type":"blockquote","content":[{"type":"attachment","attrs":{"attachmentId":FILE}}]}
    ]})
}
#[derive(Clone)]
struct Mock {
    row: Arc<Mutex<Value>>,
    uploads: Arc<Mutex<Vec<Vec<u8>>>>,
    updates: Arc<Mutex<Vec<Value>>>,
    downloads: Arc<AtomicUsize>,
    download_body: Arc<Mutex<Vec<u8>>>,
    status: Arc<AtomicU16>,
}
async fn spawn(temp: &TempDir) -> (DocmostClient, Mock, tokio::task::JoinHandle<()>) {
    let state = Mock {
        row: Arc::new(Mutex::new(row())),
        uploads: Default::default(),
        updates: Default::default(),
        downloads: Default::default(),
        download_body: Arc::new(Mutex::new(vec![1, 2, 3])),
        status: Arc::new(AtomicU16::new(200)),
    };
    let app = Router::new()
        .route(
            "/api/pages/info",
            post(|| async { Json(json!({"data":{"id":PAGE,"content":document()}})) }),
        )
        .route(
            "/api/version",
            post(|| async { Json(json!({"data":{"currentVersion":"0.95.0"}})) }),
        )
        .route(
            "/api/files/info",
            post(|State(s): State<Mock>| async move {
                Json(json!({"data":s.row.lock().unwrap().clone()}))
            }),
        )
        .route(
            "/api/files/upload",
            post(|State(s): State<Mock>, body: Bytes| async move {
                s.uploads.lock().unwrap().push(body.to_vec());
                (
                    StatusCode::from_u16(s.status.load(Ordering::SeqCst)).unwrap(),
                    Json(s.row.lock().unwrap().clone()),
                )
            }),
        )
        .route(
            "/api/pages/update",
            post(
                |State(s): State<Mock>, Json(body): Json<Value>| async move {
                    s.updates.lock().unwrap().push(body);
                    (
                        StatusCode::from_u16(s.status.load(Ordering::SeqCst)).unwrap(),
                        Json(json!({"data":{}})),
                    )
                },
            ),
        )
        .route(
            "/api/files/{id}/{name}",
            get(|State(s): State<Mock>| async move {
                s.downloads.fetch_add(1, Ordering::SeqCst);
                (
                    StatusCode::from_u16(s.status.load(Ordering::SeqCst)).unwrap(),
                    [("location", "https://example.invalid/private")],
                    s.download_body.lock().unwrap().clone(),
                )
            }),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async {
        axum::serve(listener, app).await.unwrap();
    });
    let store = StateStore::new(Some(temp.path().into()), true).unwrap();
    store
        .write_config(&StoredConfig {
            base_url: origin.clone(),
            email: "media@example.invalid".into(),
            last_authenticated_at: "2026-09-07T00:00:00Z".into(),
        })
        .await
        .unwrap();
    store
        .write_session(&StoredSession {
            origin: Some(origin.clone()),
            email: Some("media@example.invalid".into()),
            token: "synthetic-session".into(),
            expires_at: None,
            saved_at: "2026-09-07T00:00:00Z".into(),
        })
        .await
        .unwrap();
    let auth = AuthManager::new(
        StartupConfig {
            base_url: Some(origin),
            allow_insecure_loopback_http: true,
            allow_insecure_credential_file: true,
            ..Default::default()
        },
        Some(temp.path().into()),
    )
    .unwrap();
    (DocmostClient::new(auth), state, handle)
}
fn upload() -> UploadAttachmentInput {
    UploadAttachmentInput {
        page_id: PAGE.into(),
        file_name: "sample.png".into(),
        mime_type: "image/png".into(),
        data_base64: STANDARD.encode([1, 2, 3]),
    }
}
fn attachment() -> AttachmentInput {
    AttachmentInput {
        page_id: PAGE.into(),
        attachment_id: FILE.into(),
    }
}

#[tokio::test]
async fn multipart_download_and_metadata_are_bounded_and_scoped() {
    let temp = TempDir::new().unwrap();
    let (client, state, handle) = spawn(&temp).await;
    let result = client.upload_attachment(&upload()).await.unwrap();
    assert_eq!(result["id"], FILE);
    assert!(!result.to_string().contains("PRIVATE_"));
    let body = String::from_utf8(state.uploads.lock().unwrap()[0].clone()).unwrap();
    assert!(body.find("name=\"pageId\"").unwrap() < body.find("name=\"file\"").unwrap());
    assert!(body.contains("filename=\"sample.png\""));
    assert!(body.contains("Content-Type: image/png"));
    let result = client.download_attachment(&attachment()).await.unwrap();
    assert_eq!(result["data_base64"], STANDARD.encode([1, 2, 3]));
    let result = client.list_page_attachments(PAGE).await.unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    state.row.lock().unwrap()["pageId"] = json!("33333333-3333-4333-8333-333333333333");
    assert!(client.download_attachment(&attachment()).await.is_err());
    assert_eq!(state.downloads.load(Ordering::SeqCst), 1);
    handle.abort();
}

#[tokio::test]
async fn insert_uses_append_and_mutations_never_retry() {
    let temp = TempDir::new().unwrap();
    let (client, state, handle) = spawn(&temp).await;
    let input = InsertMediaInput {
        page_id: PAGE.into(),
        attachment_id: FILE.into(),
        kind: MediaKind::Image,
        alt: Some("Sample".into()),
    };
    client.append_media(&input).await.unwrap();
    let update = state.updates.lock().unwrap()[0].clone();
    assert_eq!(update["operation"], "append");
    assert_eq!(update["format"], "json");
    assert_eq!(update["content"]["content"].as_array().unwrap().len(), 1);
    assert_eq!(
        update["content"]["content"][0]["attrs"]["attachmentId"],
        FILE
    );
    for status in [401, 403, 500] {
        state.status.store(status, Ordering::SeqCst);
        let before = state.updates.lock().unwrap().len();
        assert!(client.append_media(&input).await.is_err());
        assert_eq!(state.updates.lock().unwrap().len(), before + 1);
        let before = state.uploads.lock().unwrap().len();
        let error = client
            .upload_attachment(&upload())
            .await
            .unwrap_err()
            .to_string();
        assert!(!error.contains("PRIVATE_"));
        assert_eq!(state.uploads.lock().unwrap().len(), before + 1);
    }
    state.status.store(302, Ordering::SeqCst);
    assert!(client.download_attachment(&attachment()).await.is_err());
    assert_eq!(state.downloads.load(Ordering::SeqCst), 1);
    handle.abort();
}

#[tokio::test]
async fn invalid_uploads_fail_before_transmitting() {
    let temp = TempDir::new().unwrap();
    let (client, state, handle) = spawn(&temp).await;
    for name in ["../secret", "a/b", "a\\b", "bad\r\nheader", ""] {
        let mut input = upload();
        input.file_name = name.into();
        assert!(client.upload_attachment(&input).await.is_err());
    }
    for data in [
        "!".to_string(),
        "".to_string(),
        STANDARD.encode(vec![0; media::MAX_ATTACHMENT_BYTES + 1]),
    ] {
        let mut input = upload();
        input.data_base64 = data;
        assert!(client.upload_attachment(&input).await.is_err());
    }
    assert!(state.uploads.lock().unwrap().is_empty());
    handle.abort();
}

#[tokio::test]
async fn oversized_download_is_rejected() {
    let temp = TempDir::new().unwrap();
    let (client, state, handle) = spawn(&temp).await;
    *state.download_body.lock().unwrap() = vec![0; media::MAX_ATTACHMENT_BYTES + 1];
    assert!(client.download_attachment(&attachment()).await.is_err());
    assert_eq!(state.downloads.load(Ordering::SeqCst), 1);
    handle.abort();
}

#[test]
fn rich_nodes_and_embed_validation_preserve_media_semantics() {
    assert!(media::contains_rich_media(&document()));
    assert!(!media::contains_rich_media(
        &json!({"type":"doc","content":[]})
    ));
    let file = media::media_node(&row(), &MediaKind::Attachment, None).unwrap();
    assert_eq!(file["attrs"]["mime"], "image/png");
    assert_eq!(file["attrs"]["attachmentId"], FILE);
    assert!(media::media_node(&row(), &MediaKind::Video, None).is_err());
    for url in [
        "javascript:alert(1)",
        "http://example.com",
        "https://user:pass@example.com",
        "https://example.com/\n",
    ] {
        assert!(media::embed_node(url).is_err());
    }
    assert_eq!(
        media::embed_node("https://example.com/embed").unwrap()["type"],
        "embed"
    );
    let markdown = docmost_local_mcp::prosemirror::prosemirror_to_markdown(
        &json!({"type":"doc","content":[file]}),
    );
    assert!(markdown.contains("sample.png"));
}
