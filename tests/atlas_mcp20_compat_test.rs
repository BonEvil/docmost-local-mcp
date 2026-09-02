use std::{collections::BTreeSet, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{Context, Result, bail};
use docmost_local_mcp::{
    PRODUCT_NAME, PRODUCT_TITLE, PRODUCT_VERSION, server::DocmostMcpServer, types::StartupConfig,
};
use rmcp::ServiceExt;
use serde_json::{Value, json};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

const READ_TOOL_NAMES: [&str; 10] = [
    "list_spaces",
    "search_docs",
    "search_pages",
    "get_space",
    "get_page",
    "list_pages",
    "list_child_pages",
    "get_comments",
    "list_workspace_members",
    "get_current_user",
];

struct ServerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl ServerProcess {
    fn spawn() -> Result<Self> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_docmost-local-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("spawn candidate MCP binary")?;
        let stdin = child.stdin.take().context("candidate stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("candidate stdout")?);
        Ok(Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        })
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;
        let response = self.read().await?;
        if response.get("id") != Some(&Value::from(id)) {
            bail!("candidate returned a mismatched response id");
        }
        Ok(response)
    }

    async fn notify(&mut self, method: &str) -> Result<()> {
        self.write(json!({"jsonrpc": "2.0", "method": method}))
            .await
    }

    async fn write(&mut self, value: Value) -> Result<()> {
        let mut encoded = serde_json::to_vec(&value)?;
        encoded.push(b'\n');
        self.stdin.write_all(&encoded).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read(&mut self) -> Result<Value> {
        let mut line = String::new();
        timeout(Duration::from_secs(3), self.stdout.read_line(&mut line))
            .await
            .context("candidate response timed out")??;
        if line.is_empty() {
            bail!("candidate closed stdout before responding");
        }
        serde_json::from_str(&line).context("parse candidate response")
    }

    async fn initialize(&mut self) -> Result<Value> {
        let response = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "atlas-mcp20-contract-test", "version": "1"},
                }),
            )
            .await?;
        if response.get("result").is_none() {
            bail!("initialize did not return a result");
        }
        self.notify("notifications/initialized").await?;
        Ok(response)
    }

    async fn tools(&mut self) -> Result<Vec<Value>> {
        let response = self.request("tools/list", json!({})).await?;
        response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .cloned()
            .context("tools/list result")
    }

    async fn close(mut self) -> Result<()> {
        drop(self.stdin);
        let status = timeout(Duration::from_secs(3), self.child.wait())
            .await
            .context("candidate teardown timed out")??;
        if !status.success() {
            bail!("candidate did not tear down cleanly");
        }
        Ok(())
    }
}

fn assert_read_only_inventory(tools: &[Value]) {
    let actual = tools
        .iter()
        .map(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .expect("tool name")
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    let expected = READ_TOOL_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(tools.len(), READ_TOOL_NAMES.len());
    assert!(tools.iter().all(|tool| {
        tool.pointer("/annotations/readOnlyHint")
            .and_then(Value::as_bool)
            == Some(true)
    }));
}

fn assert_product_identity(initialized: &Value) {
    assert_eq!(
        initialized
            .pointer("/result/serverInfo/name")
            .and_then(Value::as_str),
        Some(PRODUCT_NAME)
    );
    assert_eq!(
        initialized
            .pointer("/result/serverInfo/title")
            .and_then(Value::as_str),
        Some(PRODUCT_TITLE)
    );
    assert_eq!(
        initialized
            .pointer("/result/serverInfo/version")
            .and_then(Value::as_str),
        Some(PRODUCT_VERSION),
        "MCP metadata must identify the product, never the rmcp dependency"
    );
}

#[test]
fn cli_version_is_exact_product_identity() -> Result<()> {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_docmost-local-mcp"))
        .arg("--version")
        .output()
        .context("run candidate --version")?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!("{PRODUCT_NAME} {PRODUCT_VERSION}\n"),
        "binary output must not contain a stale or dependency-derived version"
    );
    assert!(output.stderr.is_empty());
    Ok(())
}

#[tokio::test]
async fn inherited_2bf01855_rmcp_boundary_closes_on_atlas_discovery() -> Result<()> {
    // Commit 2bf01855 passed stdio directly to this rmcp service path. Exercise that
    // inherited boundary without the new compatibility preflight and prove the exact
    // discovery-first frame terminates it before an initialize request can be sent.
    let (server_transport, mut atlas_transport) = tokio::io::duplex(16 * 1024);
    let server_task = tokio::spawn(async move {
        let server = DocmostMcpServer::new(StartupConfig::default())?;
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let discovery = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {"_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": {"name": "Atlas", "version": "current-contract-test"},
            "io.modelcontextprotocol/clientCapabilities": {}
        }}
    });
    atlas_transport
        .write_all(format!("{discovery}\n").as_bytes())
        .await?;
    atlas_transport.flush().await?;

    let mut response = Vec::new();
    let bytes = timeout(
        Duration::from_secs(1),
        atlas_transport.read_to_end(&mut response),
    )
    .await
    .context("inherited boundary did not close")??;
    assert_eq!(
        bytes, 0,
        "inherited boundary unexpectedly returned a response"
    );
    let result = timeout(Duration::from_secs(1), server_task)
        .await
        .context("inherited server task did not stop")??;
    let error = result.expect_err("inherited rmcp boundary unexpectedly stayed healthy");
    assert!(
        error.to_string().contains("initialized request"),
        "unexpected inherited close diagnostic: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn atlas_mcp20_add_flow_negotiates_enumerates_persists_and_tears_down() -> Result<()> {
    let mut server = ServerProcess::spawn()?;
    let discovery = server
        .request(
            "server/discover",
            json!({"_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": {
                    "name": "Atlas",
                    "version": "current-contract-test"
                },
                "io.modelcontextprotocol/clientCapabilities": {}
            }}),
        )
        .await?;
    assert_eq!(discovery.pointer("/error/code"), Some(&Value::from(-32601)));
    assert_eq!(
        discovery.pointer("/error/message").and_then(Value::as_str),
        Some("Method not found")
    );

    let initialized = server.initialize().await?;
    assert_product_identity(&initialized);
    assert_eq!(
        initialized
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str),
        Some("2025-03-26")
    );
    let tools = server.tools().await?;
    assert_read_only_inventory(&tools);

    // Model Atlas's post-enumeration commit point and prove the persisted record is
    // derived only from the negotiated identity and accepted inventory.
    let directory = tempfile::tempdir()?;
    let registration_path: PathBuf = directory.path().join("atlas-registration.json");
    let registration = json!({
        "serverInfo": initialized.pointer("/result/serverInfo"),
        "protocolVersion": initialized.pointer("/result/protocolVersion"),
        "tools": tools.iter().map(|tool| &tool["name"]).collect::<Vec<_>>(),
    });
    fs::write(&registration_path, serde_json::to_vec(&registration)?).await?;
    let persisted: Value = serde_json::from_slice(&fs::read(&registration_path).await?)?;
    assert_eq!(persisted, registration);

    server.close().await
}

#[tokio::test]
async fn ordinary_initialize_first_client_remains_standards_compliant() -> Result<()> {
    let mut server = ServerProcess::spawn()?;
    let initialized = server.initialize().await?;
    assert_product_identity(&initialized);
    assert_eq!(
        initialized
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str),
        Some("2025-03-26")
    );
    let tools = server.tools().await?;
    assert_read_only_inventory(&tools);
    server.close().await
}
