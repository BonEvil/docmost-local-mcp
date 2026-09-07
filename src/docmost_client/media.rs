use crate::{
    media::{self, AttachmentInput, InsertMediaInput, MAX_ATTACHMENT_BYTES, UploadAttachmentInput},
    network_policy::{read_bounded_body, safe_transport_error, validate_text},
};
use anyhow::{Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

impl super::DocmostClient {
    pub async fn upload_attachment(&self, input: &UploadAttachmentInput) -> Result<Value> {
        self.validate_uuid_identifier("page_id", &input.page_id)?;
        if !media::file_name_is_valid(&input.file_name) {
            bail!("Invalid attachment filename");
        }
        validate_text("mime_type", &input.mime_type, 128, false)?;
        if input.data_base64.len() > MAX_ATTACHMENT_BYTES.div_ceil(3) * 4 {
            bail!("Attachment exceeds the 2 MiB limit");
        }
        let bytes = STANDARD
            .decode(&input.data_base64)
            .map_err(|_| anyhow!("Invalid attachment base64"))?;
        if bytes.is_empty() || bytes.len() > MAX_ATTACHMENT_BYTES {
            bail!("Attachment must contain 1 byte to 2 MiB");
        }
        // Confirm that the page exists and is visible before uploading.
        self.get_page(&input.page_id)
            .await?
            .ok_or_else(|| anyhow!("Page not found"))?;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(input.file_name.clone())
            .mime_str(&input.mime_type)
            .map_err(|_| anyhow!("Invalid attachment MIME type"))?;
        let session = self.auth_manager.get_authenticated_session().await?;
        let response = self
            .http
            .post(format!("{}/api/files/upload", session.base_url))
            .bearer_auth(&session.token)
            .multipart(
                reqwest::multipart::Form::new()
                    .text("pageId", input.page_id.clone())
                    .part("file", part),
            )
            .send()
            .await
            .map_err(safe_transport_error)?;
        // Mutations are never replayed automatically, including on 401.
        let row = self.media_json(response).await?;
        self.validate_uuid_identifier("attachment_id", row["id"].as_str().unwrap_or(""))?;
        if row["pageId"].as_str() != Some(input.page_id.as_str()) {
            bail!("Attachment page mismatch");
        }
        media::attachment_summary(&row)
    }

    async fn media_json(&self, response: reqwest::Response) -> Result<Value> {
        if !response.status().is_success() {
            let status = response.status().as_u16();
            read_bounded_body(
                response,
                self.network_policy.max_error_body_bytes,
                "error response",
            )
            .await?;
            bail!("Docmost attachment request failed (HTTP {status}); no automatic retry");
        }
        let bytes = read_bounded_body(
            response,
            self.network_policy.max_success_body_bytes,
            "attachment response",
        )
        .await?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| anyhow!("Invalid attachment response"))?;
        Ok(value.get("data").cloned().unwrap_or(value))
    }

    async fn checked_attachment(&self, input: &AttachmentInput) -> Result<Value> {
        self.validate_uuid_identifier("page_id", &input.page_id)?;
        self.validate_uuid_identifier("attachment_id", &input.attachment_id)?;
        let row: Value = self
            .request(
                "/api/files/info",
                json!({"attachmentId": input.attachment_id}),
                true,
            )
            .await?;
        if row["pageId"].as_str() != Some(input.page_id.as_str())
            || row["id"].as_str() != Some(input.attachment_id.as_str())
        {
            bail!("Attachment does not belong to the requested page");
        }
        media::attachment_summary(&row)?;
        Ok(row)
    }

    pub async fn list_page_attachments(&self, page_id: &str) -> Result<Value> {
        let page = self
            .get_page(page_id)
            .await?
            .ok_or_else(|| anyhow!("Page not found"))?;
        let id = page.id.ok_or_else(|| anyhow!("Page ID missing"))?;
        let content = page
            .content
            .ok_or_else(|| anyhow!("Page content missing"))?;
        let mut ids = std::collections::BTreeSet::new();
        media::attachment_ids(&content, &mut ids);
        if ids.len() > 100 {
            bail!("Page contains more than 100 attachment references; use get_page_content");
        }
        let mut items = Vec::new();
        for attachment_id in ids {
            let row = self
                .checked_attachment(&AttachmentInput {
                    page_id: id.clone(),
                    attachment_id,
                })
                .await?;
            items.push(media::attachment_summary(&row)?);
        }
        Ok(
            json!({"items": items, "scope": "Referenced attachments only; unattached uploads are not listed"}),
        )
    }

    pub async fn download_attachment(&self, input: &AttachmentInput) -> Result<Value> {
        let row = self.checked_attachment(input).await?;
        let mut summary = media::attachment_summary(&row)?;
        let session = self.auth_manager.get_authenticated_session().await?;
        let response = self
            .http
            .get(format!(
                "{}{}",
                session.base_url,
                summary["url"].as_str().unwrap()
            ))
            .bearer_auth(&session.token)
            .send()
            .await
            .map_err(safe_transport_error)?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            read_bounded_body(
                response,
                self.network_policy.max_error_body_bytes,
                "error response",
            )
            .await?;
            bail!("Attachment download failed (HTTP {status})");
        }
        let bytes =
            read_bounded_body(response, MAX_ATTACHMENT_BYTES, "attachment download").await?;
        summary["data_base64"] = Value::String(STANDARD.encode(bytes));
        Ok(summary)
    }

    pub async fn append_media(&self, input: &InsertMediaInput) -> Result<()> {
        let row = self
            .checked_attachment(&AttachmentInput {
                page_id: input.page_id.clone(),
                attachment_id: input.attachment_id.clone(),
            })
            .await?;
        if let Some(alt) = &input.alt {
            validate_text("alt", alt, 4096, true)?;
        }
        self.append_node(
            &input.page_id,
            media::media_node(&row, &input.kind, input.alt.as_deref())?,
        )
        .await
    }

    pub async fn append_node(&self, page_id: &str, node: Value) -> Result<()> {
        self.validate_uuid_identifier("page_id", page_id)?;
        if !self.capabilities().await.rest_page_body_update {
            bail!("Media insertion requires confirmed Docmost v0.70.0 or newer");
        }
        self.request_discard_with_retry(
            "/api/pages/update",
            json!({
                "pageId": page_id, "content": {"type": "doc", "content": [node]},
                "operation": "append", "format": "json"
            }),
            false,
        )
        .await
    }
}
