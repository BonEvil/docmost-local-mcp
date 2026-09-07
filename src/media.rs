//! Typed rich-media operations. Upload bytes are explicit, never arbitrary local paths.
use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

pub const MAX_ATTACHMENT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct UploadAttachmentInput {
    pub page_id: String,
    /// A basename, not a path.
    pub file_name: String,
    pub mime_type: String,
    /// Standard base64; maximum decoded size 2 MiB. Never interpreted as a path or URL.
    pub data_base64: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AttachmentInput {
    pub page_id: String,
    pub attachment_id: String,
}

pub fn content_hash(value: &Value) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("JSON value"))
    )
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
    Attachment,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InsertMediaInput {
    pub page_id: String,
    /// ID of a file already uploaded to this same page.
    pub attachment_id: String,
    pub kind: MediaKind,
    #[serde(default)]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InsertEmbedInput {
    pub page_id: String,
    /// HTTPS embed URL. The connector stores it but never fetches it.
    pub url: String,
}

pub fn file_name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | '"' | ';'))
}

/// Project only public attachment metadata; never expose storage paths or creator IDs.
pub fn attachment_summary(row: &Value) -> Result<Value> {
    let id = row["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Attachment ID missing"))?;
    let name = row["fileName"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Attachment filename missing"))?;
    if !file_name_is_valid(name) {
        bail!("Invalid attachment filename");
    }
    let mime = row["mimeType"]
        .as_str()
        .filter(|mime| mime.len() <= 128)
        .ok_or_else(|| anyhow::anyhow!("Invalid attachment MIME metadata"))?;
    let page = row["pageId"]
        .as_str()
        .filter(|page| page.len() == 36)
        .ok_or_else(|| anyhow::anyhow!("Invalid attachment page metadata"))?;
    if id.len() != 36 {
        bail!("Invalid attachment ID metadata");
    }
    let size = row["fileSize"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid attachment size metadata"))?;
    Ok(json!({
        "id": id, "file_name": name, "mime_type": mime,
        "size": size, "page_id": page,
        "url": format!("/api/files/{}/{}", urlencoding::encode(id), urlencoding::encode(name))
    }))
}

pub fn media_node(row: &Value, kind: &MediaKind, alt: Option<&str>) -> Result<Value> {
    let summary = attachment_summary(row)?;
    let mime = row["mimeType"].as_str().unwrap_or("");
    let node = match kind {
        MediaKind::Image if mime.starts_with("image/") => "image",
        MediaKind::Video if mime.starts_with("video/") => "video",
        MediaKind::Attachment => "attachment",
        _ => bail!("Attachment MIME type does not match the requested media kind"),
    };
    let mut attrs = json!({"attachmentId": row["id"], "size": row["fileSize"]});
    if node == "attachment" {
        attrs["url"] = summary["url"].clone();
        attrs["name"] = row["fileName"].clone();
        attrs["mime"] = row["mimeType"].clone();
    } else {
        attrs["src"] = summary["url"].clone();
        attrs["alt"] = Value::String(alt.unwrap_or("").to_string());
    }
    Ok(json!({"type": node, "attrs": attrs}))
}

pub fn embed_node(raw: &str) -> Result<Value> {
    let url = url::Url::parse(raw).map_err(|_| anyhow::anyhow!("Invalid embed URL"))?;
    if raw.len() > 4096
        || raw.chars().any(char::is_control)
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("Embed URL must be HTTPS without credentials");
    }
    Ok(json!({"type": "embed", "attrs": {"src": url.as_str(), "provider": "iframe"}}))
}

pub fn attachment_ids(value: &Value, ids: &mut std::collections::BTreeSet<String>) {
    if let Some(id) = value.pointer("/attrs/attachmentId").and_then(Value::as_str) {
        ids.insert(id.to_owned());
    }
    if let Some(children) = value["content"].as_array() {
        for child in children {
            attachment_ids(child, ids);
        }
    }
}

pub fn contains_rich_media(value: &Value) -> bool {
    matches!(
        value["type"].as_str(),
        Some("image" | "video" | "attachment" | "embed" | "drawio" | "excalidraw")
    ) || value.pointer("/attrs/attachmentId").is_some()
        || value["content"]
            .as_array()
            .is_some_and(|c| c.iter().any(contains_rich_media))
}
