use crate::{
    media::{self, AttachmentInput, InsertEmbedInput, InsertMediaInput, UploadAttachmentInput},
    server::{DocmostMcpServer, internal_error},
    types::GetPageInput,
};
use rmcp::{handler::server::wrapper::Parameters, model::ErrorData, tool, tool_router};

impl DocmostMcpServer {
    fn media_output(&self, value: serde_json::Value) -> anyhow::Result<String> {
        let output = value.to_string();
        crate::network_policy::validate_text(
            "tool output",
            &output,
            self.client.network_policy().max_tool_output_bytes,
            true,
        )?;
        Ok(output)
    }
}

#[tool_router(router = media_read_tool_router, vis = "pub(crate)")]
impl DocmostMcpServer {
    #[tool(
        name = "get_page_content",
        description = "Read the complete ProseMirror JSON document with rich-media nodes and a content SHA-256. Use this instead of Markdown for lossless inspection.",
        annotations(read_only_hint = true)
    )]
    async fn get_page_content(
        &self,
        Parameters(input): Parameters<GetPageInput>,
    ) -> Result<String, ErrorData> {
        let page = self
            .client
            .get_page(&input.slug_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| internal_error(anyhow::anyhow!("Page not found")))?;
        let content = page
            .content
            .ok_or_else(|| internal_error(anyhow::anyhow!("Page content missing")))?;
        let output = serde_json::json!({"page_id": page.id, "content_sha256": media::content_hash(&content), "content": content}).to_string();
        crate::network_policy::validate_text(
            "tool output",
            &output,
            self.client.network_policy().max_tool_output_bytes,
            true,
        )
        .map_err(internal_error)?;
        Ok(output)
    }

    #[tool(
        name = "list_page_attachments",
        description = "List metadata for up to 100 distinct attachments referenced in a page, including nested media. Does not enumerate unattached uploads.",
        annotations(read_only_hint = true)
    )]
    async fn list_page_attachments(
        &self,
        Parameters(input): Parameters<GetPageInput>,
    ) -> Result<String, ErrorData> {
        self.client
            .list_page_attachments(&input.slug_id)
            .await
            .and_then(|v| self.media_output(v))
            .map_err(internal_error)
    }

    #[tool(
        name = "download_attachment",
        description = "Read an attachment belonging to the specified page as base64, with metadata. Maximum 2 MiB; no filesystem writes or arbitrary URLs.",
        annotations(read_only_hint = true)
    )]
    async fn download_attachment(
        &self,
        Parameters(input): Parameters<AttachmentInput>,
    ) -> Result<String, ErrorData> {
        self.client
            .download_attachment(&input)
            .await
            .and_then(|v| self.media_output(v))
            .map_err(internal_error)
    }
}

#[tool_router(router = media_write_tool_router, vis = "pub(crate)")]
impl DocmostMcpServer {
    #[tool(
        name = "upload_attachment",
        description = "Upload explicit base64 bytes (maximum 2 MiB) to a Docmost page. Returns attachment metadata; use insert_media to add it to the document. No automatic retries.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn upload_attachment(
        &self,
        Parameters(input): Parameters<UploadAttachmentInput>,
    ) -> Result<String, ErrorData> {
        self.client
            .upload_attachment(&input)
            .await
            .and_then(|v| self.media_output(v))
            .map_err(internal_error)
    }

    #[tool(
        name = "insert_media",
        description = "Append an uploaded image, video, or file attachment to its owning page while preserving existing rich content. Does not upload or fetch external URLs. No automatic retries.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn insert_media(
        &self,
        Parameters(input): Parameters<InsertMediaInput>,
    ) -> Result<String, ErrorData> {
        self.client
            .append_media(&input)
            .await
            .map_err(internal_error)?;
        Ok("Appended media to the page.".into())
    }

    #[tool(
        name = "insert_embed",
        description = "Append an HTTPS iframe embed URL without fetching it. Existing page content is preserved; the external provider must permit embedding. No automatic retries.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn insert_embed(
        &self,
        Parameters(input): Parameters<InsertEmbedInput>,
    ) -> Result<String, ErrorData> {
        let node = media::embed_node(&input.url).map_err(internal_error)?;
        self.client
            .append_node(&input.page_id, node)
            .await
            .map_err(internal_error)?;
        Ok("Appended embed to the page.".into())
    }
}
