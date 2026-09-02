use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{ErrorData, ServerCapabilities, ServerInfo},
    tool_handler,
};

use crate::docmost_client::DocmostClient;

mod render;
mod tools;
mod tools_delete;
mod tools_page_write;
mod tools_write;

#[derive(Debug, Clone)]
pub struct DocmostMcpServer {
    client: DocmostClient,
    tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for DocmostMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Docmost MCP server for listing spaces, searching docs, and fetching pages. Persistent mutations are exposed only when the operator explicitly starts the server in write mode with a per-tool allowlist. MCP annotations are advisory and do not replace client-side confirmation controls."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn internal_error(error: anyhow::Error) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}
