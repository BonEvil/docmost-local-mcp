//! Run ONLY against a fresh disposable Docmost v0.95.0 listening on loopback.
//! This test deliberately creates a synthetic workspace and pages.
use base64::{Engine, engine::general_purpose::STANDARD};
use docmost_local_mcp::{
    auth::manager::AuthManager,
    docmost_client::DocmostClient,
    media,
    server::DocmostMcpServer,
    storage::state_store::StateStore,
    types::{AuthorityMode, StartupConfig, StoredConfig, StoredSession},
};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParam, ClientInfo},
};
use serde_json::json;
use tempfile::TempDir;

#[derive(Debug, Clone, Default)]
struct TestClient;
impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}
async fn call(
    mcp: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    name: &str,
    args: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let result = mcp
        .call_tool(CallToolRequestParam {
            name: name.to_string().into(),
            arguments: args.as_object().cloned(),
        })
        .await?;
    assert_ne!(result.is_error, Some(true), "MCP {name} failed");
    let projected = serde_json::to_value(result)?;
    let text = projected["content"][0]["text"].as_str().expect("tool text");
    Ok(serde_json::from_str(text).unwrap_or_else(|_| json!(text)))
}

#[tokio::test]
#[ignore = "requires an empty disposable Docmost v0.95.0; creates synthetic data"]
async fn media_roundtrip_on_disposable_docmost() -> anyhow::Result<()> {
    assert_eq!(
        std::env::var("DOCMOST_MEDIA_DISPOSABLE").as_deref(),
        Ok("yes")
    );
    let origin = "http://127.0.0.1:3000";
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let response = http.post(format!("{origin}/api/auth/setup")).json(&json!({
        "name":"Media Regression","email":"media@example.invalid",
        "password":"Disposable-regression-only-729!","workspaceName":"Disposable Media Regression"
    })).send().await?;
    assert!(
        response.status().is_success(),
        "disposable setup failed: {}",
        response.status()
    );
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .find_map(|header| {
            let value = header.to_str().ok()?;
            let (name, token) = value.split(';').next()?.split_once('=')?;
            (name == "authToken").then(|| token.to_string())
        })
        .expect("setup auth cookie");
    let temp = TempDir::new()?;
    let store = StateStore::new(Some(temp.path().into()), true)?;
    store
        .write_config(&StoredConfig {
            base_url: origin.into(),
            email: "media@example.invalid".into(),
            last_authenticated_at: "2026-09-07T00:00:00Z".into(),
        })
        .await?;
    store
        .write_session(&StoredSession {
            origin: Some(origin.into()),
            email: Some("media@example.invalid".into()),
            token: cookie,
            expires_at: None,
            saved_at: "2026-09-07T00:00:00Z".into(),
        })
        .await?;
    let auth = AuthManager::new(
        StartupConfig {
            base_url: Some(origin.into()),
            allow_insecure_loopback_http: true,
            allow_insecure_credential_file: true,
            ..Default::default()
        },
        Some(temp.path().into()),
    )?;
    let client = DocmostClient::new(auth);
    let config = StartupConfig {
        base_url: Some(origin.into()),
        allow_insecure_loopback_http: true,
        allow_insecure_credential_file: true,
        authority_mode: AuthorityMode::Write,
        allowed_write_tools: [
            "upload_attachment",
            "insert_media",
            "insert_embed",
            "update_page",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
    };
    let server = DocmostMcpServer::new_with_state_dir(config, Some(temp.path().into()))?;
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let server_handle = tokio::spawn(async move {
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let mcp = TestClient.serve(client_transport).await?;
    let space = client
        .create_space("Media regression", "media-regression", None)
        .await?;
    let page = client
        .create_page(
            &space.id,
            "Media regression",
            Some("Original text must survive."),
            None,
        )
        .await?;
    let id = page.id.expect("page id");
    let mut before = client
        .get_page(&id)
        .await?
        .expect("page")
        .content
        .expect("content");
    // Tiny PNG and a text file are synthetic fixtures, not production attachments.
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jF1kAAAAASUVORK5CYII=";
    for (name, mime, data, kind) in [
        ("pixel.png", "image/png", png.to_string(), "image"),
        (
            "note.txt",
            "text/plain",
            STANDARD.encode("Synthetic attachment"),
            "attachment",
        ),
        (
            "clip.mp4",
            "video/mp4",
            STANDARD.encode(b"synthetic-video-node-fixture"),
            "video",
        ),
    ] {
        let uploaded = call(
            &mcp,
            "upload_attachment",
            json!({
                "page_id":id,"file_name":name,"mime_type":mime,"data_base64":data
            }),
        )
        .await?;
        let attachment_id = uploaded["id"].as_str().expect("attachment id").to_string();
        let downloaded = call(
            &mcp,
            "download_attachment",
            json!({
                "page_id":id,"attachment_id":attachment_id
            }),
        )
        .await?;
        assert_eq!(downloaded["data_base64"], data);
        call(
            &mcp,
            "insert_media",
            json!({
                "page_id":id,"attachment_id":attachment_id,"kind":kind,"alt":"Synthetic fixture"
            }),
        )
        .await?;
    }
    call(
        &mcp,
        "insert_embed",
        json!({"page_id":id,"url":"https://example.com/embed"}),
    )
    .await?;
    // REST updates use the collaborative service; allow bounded persistence delay.
    let mut after = json!(null);
    for _ in 0..20 {
        after = client
            .get_page(&id)
            .await?
            .expect("page")
            .content
            .expect("content");
        if after["content"]
            .as_array()
            .is_some_and(|nodes| nodes.iter().any(|n| n["type"] == "embed"))
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    // Docmost's collaborative service omits null textAlign on its first update.
    // Normalize only this verified no-op default, retaining IDs and all other attrs.
    for node in before["content"].as_array_mut().expect("original document") {
        if node.pointer("/attrs/textAlign") == Some(&serde_json::Value::Null) {
            node["attrs"].as_object_mut().unwrap().remove("textAlign");
        }
    }
    let nodes = after["content"].as_array().expect("document");
    for node in before["content"].as_array().expect("original document") {
        assert!(nodes.contains(node), "original node was changed or lost");
    }
    for kind in ["image", "attachment", "video", "embed"] {
        assert!(
            nodes.iter().any(|n| n["type"] == kind),
            "missing {kind} node"
        );
    }
    assert_eq!(
        client.list_page_attachments(&id).await?["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(media::contains_rich_media(&after));
    let rich = call(&mcp, "get_page_content", json!({"slug_id":id})).await?;
    assert_eq!(rich["content"], after);
    assert_eq!(
        call(&mcp, "list_page_attachments", json!({"slug_id":id})).await?["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let denied = mcp
        .call_tool(CallToolRequestParam {
            name: "update_page".into(),
            arguments: json!({"page_id":id,"markdown":"Would erase media"})
                .as_object()
                .cloned(),
        })
        .await;
    assert!(
        denied.is_err() || denied.unwrap().is_error == Some(true),
        "Markdown replacement must fail closed"
    );
    assert_eq!(client.get_page(&id).await?.unwrap().content.unwrap(), after);
    mcp.cancel().await?;
    server_handle.await??;
    // Container/volume teardown removes the entire disposable workspace after the run.
    Ok(())
}
