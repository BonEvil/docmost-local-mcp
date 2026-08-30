//! Fail-closed network and content limits shared by authentication and API calls.

use std::time::Duration;

use anyhow::{Result, bail};
use reqwest::{Client, Response, redirect::Policy};

/// Production defaults. These are intentionally code-owned rather than environment
/// tunables so an Atlas launch cannot accidentally weaken the reviewed policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_success_body_bytes: usize,
    pub max_error_body_bytes: usize,
    pub max_markdown_bytes: usize,
    pub max_tool_output_bytes: usize,
    pub max_structured_content_bytes: usize,
    pub max_search_bytes: usize,
    pub max_identifier_bytes: usize,
    pub max_cursor_bytes: usize,
    pub max_title_bytes: usize,
    pub max_description_bytes: usize,
    pub max_list_limit: u32,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_success_body_bytes: 8 * 1024 * 1024,
            max_error_body_bytes: 64 * 1024,
            max_markdown_bytes: 4 * 1024 * 1024,
            max_tool_output_bytes: 4 * 1024 * 1024,
            max_structured_content_bytes: 4 * 1024 * 1024,
            max_search_bytes: 4 * 1024,
            max_identifier_bytes: 4 * 1024,
            max_cursor_bytes: 4 * 1024,
            max_title_bytes: 1024,
            max_description_bytes: 16 * 1024,
            max_list_limit: 100,
        }
    }
}

impl NetworkPolicy {
    /// A client which never follows redirects. Refusing every redirect is deliberately
    /// stricter than reqwest's cross-origin header stripping and keeps authentication
    /// material pinned to the canonical origin selected by `AuthManager`.
    pub fn build_http_client(self) -> Result<Client> {
        Ok(Client::builder()
            .connect_timeout(self.connect_timeout)
            .timeout(self.request_timeout)
            .redirect(Policy::none())
            .build()?)
    }
}

/// Consume a response body with both a Content-Length preflight and a streaming hard cap.
/// The latter is authoritative for chunked or dishonest responses.
pub async fn read_bounded_body(
    mut response: Response,
    limit: usize,
    body_class: &str,
) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|declared| declared > limit as u64)
    {
        bail!("Docmost {body_class} exceeded the {limit}-byte limit.");
    }

    let mut body =
        Vec::with_capacity(response.content_length().unwrap_or(0).min(limit as u64) as usize);
    while let Some(chunk) = response.chunk().await.map_err(safe_transport_error)? {
        if body.len().saturating_add(chunk.len()) > limit {
            bail!("Docmost {body_class} exceeded the {limit}-byte limit.");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Convert reqwest failures to content-free, URL-free deterministic classes. Keeping the
/// original reqwest error in an anyhow chain would project its request URL at the process
/// boundary when errors are printed with their full context.
pub fn safe_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::anyhow!("The Docmost request timed out.")
    } else if error.is_connect() {
        anyhow::anyhow!("The Docmost connection could not be established.")
    } else if error.is_body() || error.is_decode() {
        anyhow::anyhow!("The Docmost response body could not be read.")
    } else {
        anyhow::anyhow!("The Docmost request failed.")
    }
}

pub fn validate_text(name: &str, value: &str, max_bytes: usize, allow_empty: bool) -> Result<()> {
    if !allow_empty && value.trim().is_empty() {
        bail!("{name} must not be empty.");
    }
    if value.len() > max_bytes {
        bail!("{name} exceeded the {max_bytes}-byte limit.");
    }
    Ok(())
}

pub fn validate_optional_text(
    name: &str,
    value: Option<&str>,
    max_bytes: usize,
    allow_empty: bool,
) -> Result<()> {
    if let Some(value) = value {
        validate_text(name, value, max_bytes, allow_empty)?;
    }
    Ok(())
}

pub fn validate_limit(limit: Option<u32>, maximum: u32) -> Result<()> {
    if let Some(limit) = limit
        && (limit == 0 || limit > maximum)
    {
        bail!("limit must be between 1 and {maximum}.");
    }
    Ok(())
}

pub fn validate_json_size(name: &str, value: &serde_json::Value, max_bytes: usize) -> Result<()> {
    let size = serde_json::to_vec(value)?.len();
    if size > max_bytes {
        bail!("{name} exceeded the {max_bytes}-byte serialized limit.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::NetworkPolicy;

    #[test]
    fn production_policy_matches_documented_fail_closed_defaults() {
        let policy = NetworkPolicy::default();
        assert_eq!(policy.connect_timeout, Duration::from_secs(5));
        assert_eq!(policy.request_timeout, Duration::from_secs(30));
        assert_eq!(policy.max_success_body_bytes, 8 * 1024 * 1024);
        assert_eq!(policy.max_error_body_bytes, 64 * 1024);
        assert_eq!(policy.max_markdown_bytes, 4 * 1024 * 1024);
        assert_eq!(policy.max_tool_output_bytes, 4 * 1024 * 1024);
        assert_eq!(policy.max_structured_content_bytes, 4 * 1024 * 1024);
        assert_eq!(policy.max_search_bytes, 4 * 1024);
        assert_eq!(policy.max_identifier_bytes, 4 * 1024);
        assert_eq!(policy.max_cursor_bytes, 4 * 1024);
        assert_eq!(policy.max_title_bytes, 1024);
        assert_eq!(policy.max_description_bytes, 16 * 1024);
        assert_eq!(policy.max_list_limit, 100);
    }
}
