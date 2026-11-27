use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{ErrorData, Implementation, ServerCapabilities, ServerInfo},
};

use crate::{PRODUCT_NAME, PRODUCT_TITLE, PRODUCT_VERSION, docmost_client::DocmostClient};

mod render;
mod tools;
mod tools_delete;
mod tools_media;
mod tools_page_write;
mod tools_write;

#[derive(Debug, Clone)]
pub struct DocmostMcpServer {
    client: DocmostClient,
    tool_router: ToolRouter<Self>,
}

impl ServerHandler for DocmostMcpServer {
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        let call = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        match self.tool_router.call(call).await {
            // Domain failures belong in the tool result, not the JSON-RPC transport.
            // Invalid arguments and unknown tool names retain protocol error semantics.
            Err(error) if error.code == rmcp::model::ErrorCode::INTERNAL_ERROR => {
                Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::Content::text(error.message),
                ]))
            }
            result => result,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, ErrorData> {
        Ok(rmcp::model::ListToolsResult::with_all_items(
            self.tool_router.list_all(),
        ))
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Docmost MCP server for listing spaces, searching docs, and fetching pages. Persistent mutations are exposed only when the operator explicitly starts the server in write mode with a per-tool allowlist. MCP annotations are advisory and do not replace client-side confirmation controls."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: PRODUCT_NAME.to_owned(),
                title: Some(PRODUCT_TITLE.to_owned()),
                version: PRODUCT_VERSION.to_owned(),
                icons: None,
                website_url: Some(env!("CARGO_PKG_HOMEPAGE").to_owned()),
            },
            ..Default::default()
        }
    }
}

fn internal_error(error: anyhow::Error) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}
