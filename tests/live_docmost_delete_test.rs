//! Opt-in disposable Docmost Community v0.95.0 delete verification.
//!
//! This test refuses non-loopback targets and requires an explicit disposable sentinel.
//! It is ignored by ordinary test runs because it performs real destructive mutations.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use docmost_local_mcp::{
    auth::manager::AuthManager,
    docmost_client::DocmostClient,
    prosemirror::markdown_to_prosemirror,
    server::DocmostMcpServer,
    types::{AuthorityMode, LoginInput, StartupConfig},
};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParam, ClientInfo},
};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Default)]
struct DummyClientHandler;

impl ClientHandler for DummyClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

async fn raw_post(
    http: &reqwest::Client,
    base_url: &str,
    token: &str,
    endpoint: &str,
    body: Value,
) -> Result<reqwest::Response> {
    Ok(http
        .post(format!("{base_url}{endpoint}"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?)
}

async fn call_delete(
    mcp: &rmcp::service::RunningService<rmcp::RoleClient, DummyClientHandler>,
    name: &str,
    field: &str,
    id: &str,
) -> Result<Value> {
    let mut arguments = Map::new();
    arguments.insert(field.to_string(), Value::String(id.to_string()));
    let result = mcp
        .call_tool(CallToolRequestParam {
            name: name.to_string().into(),
            arguments: Some(arguments),
        })
        .await?;
    if result.is_error != Some(false) {
        bail!("{name} returned an MCP error");
    }
    let structured = result
        .structured_content
        .context("delete result must use MCP structured_content")?;
    if structured["target"]["id"] != id
        || structured["automaticRetry"] != false
        || structured["consequence"].is_null()
    {
        bail!("{name} returned incomplete target/consequence/retry context");
    }
    Ok(structured)
}

#[tokio::test]
#[ignore = "requires explicitly authorized disposable Docmost v0.95.0 infrastructure"]
async fn every_delete_tool_has_an_independently_observed_disposable_effect() -> Result<()> {
    if std::env::var("DOCMOST_LIVE_DELETE_DISPOSABLE").as_deref() != Ok("1") {
        bail!("DOCMOST_LIVE_DELETE_DISPOSABLE=1 is required");
    }
    let base_url = std::env::var("DOCMOST_LIVE_DELETE_BASE_URL")?;
    if !base_url.starts_with("http://127.0.0.1:") {
        bail!("live delete verification refuses non-loopback targets");
    }
    let email = std::env::var("DOCMOST_LIVE_DELETE_EMAIL")?;
    let password = std::env::var("DOCMOST_LIVE_DELETE_PASSWORD")?;
    let suffix = std::env::var("DOCMOST_LIVE_DELETE_SUFFIX")?;
    if !suffix
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
    {
        bail!("synthetic suffix must be alphanumeric");
    }

    unsafe { std::env::set_var("DOCMOST_DISABLE_KEYRING", "1") };
    let base_config = StartupConfig {
        base_url: Some(base_url.clone()),
        allow_insecure_loopback_http: true,
        allow_insecure_credential_file: true,
        ..StartupConfig::default()
    };
    let auth = AuthManager::new(base_config.clone(), None)?;
    let session = auth
        .login(LoginInput {
            base_url: base_url.clone(),
            email,
            password,
            remember_password: false,
        })
        .await?;
    let direct = DocmostClient::new(auth);
    let http = reqwest::Client::new();

    let mut write_config = base_config;
    write_config.authority_mode = AuthorityMode::Write;
    write_config.allowed_write_tools = BTreeSet::from([
        "delete_page".to_string(),
        "delete_space".to_string(),
        "delete_comment".to_string(),
    ]);
    let (server_transport, client_transport) = tokio::io::duplex(32 * 1024);
    let server_handle = tokio::spawn(async move {
        let server = DocmostMcpServer::new(write_config)?;
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let mcp = DummyClientHandler.serve(client_transport).await?;

    let mut cleanup_space_ids = Vec::new();
    let verification: Result<()> = async {
        let page_space = direct
            .create_space(
                "Synthetic page delete space",
                &format!("deletepage{suffix}"),
                Some("Disposable v0.95.0 page-delete verification"),
            )
            .await?;
        cleanup_space_ids.push(page_space.id.clone());
        let parent = direct
            .create_page(&page_space.id, "Synthetic delete parent", None, None)
            .await?;
        let parent_id = parent.id.context("created parent missing UUID")?;
        let child = direct
            .create_page(
                &page_space.id,
                "Synthetic delete child",
                None,
                Some(&parent_id),
            )
            .await?;
        let child_id = child.id.context("created child missing UUID")?;
        let page_result = call_delete(&mcp, "delete_page", "page_id", &parent_id).await?;
        if page_result["outcome"] != "moved_to_trash"
            || page_result["consequence"]["permanent"] != false
        {
            bail!("delete_page structured outcome was incorrect");
        }
        for page_id in [&parent_id, &child_id] {
            let response = raw_post(
                &http,
                &base_url,
                &session.token,
                "/api/pages/info",
                json!({"pageId": page_id}),
            )
            .await?;
            let status = response.status();
            let body: Value = response.json().await?;
            if !status.is_success() || body["data"]["deletedAt"].is_null() {
                bail!("delete_page did not trash the exact target and descendant");
            }
        }

        let comment_space = direct
            .create_space(
                "Synthetic comment delete space",
                &format!("deletecomment{suffix}"),
                Some("Disposable v0.95.0 comment-delete verification"),
            )
            .await?;
        cleanup_space_ids.push(comment_space.id.clone());
        let comment_page = direct
            .create_page(
                &comment_space.id,
                "Synthetic threaded comment page",
                None,
                None,
            )
            .await?;
        let comment_page_id = comment_page.id.context("comment page missing UUID")?;
        let parent_comment = direct
            .create_comment(
                &comment_page_id,
                &markdown_to_prosemirror("Synthetic parent comment"),
            )
            .await?;
        let reply_response = raw_post(
            &http,
            &base_url,
            &session.token,
            "/api/comments/create",
            json!({
                "pageId": comment_page_id,
                "content": serde_json::to_string(&markdown_to_prosemirror("Synthetic reply"))?,
                "parentCommentId": parent_comment.id
            }),
        )
        .await?;
        if !reply_response.status().is_success() {
            bail!("failed to create disposable threaded reply");
        }
        let reply_body: Value = reply_response.json().await?;
        let reply_id = reply_body["data"]["id"]
            .as_str()
            .context("reply missing UUID")?
            .to_string();
        let comment_result =
            call_delete(&mcp, "delete_comment", "comment_id", &parent_comment.id).await?;
        if comment_result["outcome"] != "permanently_deleted"
            || comment_result["consequence"]["cascade"]
                .as_array()
                .is_none_or(|items| !items.iter().any(|item| item == "threaded_replies"))
        {
            bail!("delete_comment structured outcome was incorrect");
        }
        for comment_id in [&parent_comment.id, &reply_id] {
            let response = raw_post(
                &http,
                &base_url,
                &session.token,
                "/api/comments/info",
                json!({"commentId": comment_id}),
            )
            .await?;
            if response.status() != reqwest::StatusCode::NOT_FOUND {
                bail!("delete_comment did not remove target and threaded reply");
            }
        }

        let space = direct
            .create_space(
                "Synthetic permanent delete space",
                &format!("deletespace{suffix}"),
                Some("Disposable v0.95.0 space-delete verification"),
            )
            .await?;
        cleanup_space_ids.push(space.id.clone());
        let space_page = direct
            .create_page(&space.id, "Synthetic cascade page", None, None)
            .await?;
        let space_page_id = space_page.id.context("space page missing UUID")?;
        let space_comment = direct
            .create_comment(
                &space_page_id,
                &markdown_to_prosemirror("Synthetic cascade comment"),
            )
            .await?;
        let space_result = call_delete(&mcp, "delete_space", "space_id", &space.id).await?;
        if space_result["outcome"] != "permanently_deleted"
            || space_result["consequence"]["asynchronousFollowUp"]
                != "attachment_cleanup_queued_by_docmost"
        {
            bail!("delete_space structured outcome was incorrect");
        }
        for (endpoint, body) in [
            ("/api/spaces/info", json!({"spaceId": space.id})),
            ("/api/pages/info", json!({"pageId": space_page_id})),
            ("/api/comments/info", json!({"commentId": space_comment.id})),
        ] {
            let response = raw_post(&http, &base_url, &session.token, endpoint, body).await?;
            if response.status() != reqwest::StatusCode::NOT_FOUND {
                bail!("delete_space did not cascade the independently read target");
            }
        }
        Ok(())
    }
    .await;

    for space_id in &cleanup_space_ids {
        let _ = direct.delete_space(space_id).await;
    }
    mcp.cancel().await?;
    server_handle.await??;
    verification
}
