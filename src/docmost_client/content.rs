//! Lossless body edits with snapshot and media-preservation guards.
use crate::{media, network_policy::validate_json_size, types::DocmostPage};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

impl super::DocmostClient {
    pub async fn update_page_content(
        &self,
        page_id: &str,
        title: Option<&str>,
        content: &Value,
        expected_hash: &str,
    ) -> Result<DocmostPage> {
        self.validate_identifier("page_id", page_id)?;
        crate::network_policy::validate_optional_text(
            "title",
            title,
            self.network_policy.max_title_bytes,
            false,
        )?;
        validate_json_size(
            "page content",
            content,
            self.network_policy.max_structured_content_bytes,
        )?;
        if expected_hash.len() != 64 || !expected_hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            bail!("expected_content_sha256 must be the hash returned by get_page_content");
        }
        if !self.capabilities().await.rest_page_body_update {
            bail!(
                "Structured body edits require confirmed REST body-update support (Docmost v0.70.0+)"
            );
        }
        let page = self
            .get_page(page_id)
            .await?
            .ok_or_else(|| anyhow!("Page not found"))?;
        let existing = page
            .content
            .as_ref()
            .ok_or_else(|| anyhow!("Page content missing"))?;
        if media::content_hash(existing) != expected_hash {
            bail!(
                "Page content changed; read get_page_content again before editing. No write performed"
            );
        }
        media::validate_preserving_media(existing, content)?;
        let id = page.id.ok_or_else(|| anyhow!("Page ID missing"))?;
        // This is an inspection guard, not server-side CAS. Never replay a replacement
        // after a 401 or ambiguous failure. The caller must inspect durable state.
        let mut payload = json!({
            "pageId": id, "content": content, "operation": "replace", "format": "json"
        });
        if let Some(title) = title {
            payload["title"] = json!(title);
        }
        self.request("/api/pages/update", payload, false).await
    }
}
