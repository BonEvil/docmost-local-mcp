use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::OnceCell;

use crate::{
    auth::manager::{AuthManager, safe_read_response_text},
    debug::debug_log,
    network_policy::{
        NetworkPolicy, read_bounded_body, safe_transport_error, validate_limit,
        validate_optional_text, validate_text,
    },
    types::{
        DocmostComment, DocmostCurrentUserResponse, DocmostPage, DocmostPageListItem,
        DocmostSearchResult, DocmostSpace, DocmostSpaceWithMembership, DocmostUser,
    },
    version::{Capabilities, ServerVersion, VersionResponse},
};

#[derive(Debug, Clone)]
pub struct DocmostClient {
    auth_manager: AuthManager,
    http: Client,
    network_policy: NetworkPolicy,
    /// Detected server version, fetched once (on success) and shared across clones.
    version: Arc<OnceCell<ServerVersion>>,
}

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, serde::Deserialize)]
struct ApiEnvelope<T> {
    data: Option<T>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ListResult<T> {
    List(Vec<T>),
    Wrapped { items: Option<Vec<T>> },
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorListResult<T> {
    pub items: Option<Vec<T>>,
}

mod writes;

impl DocmostClient {
    pub fn new(auth_manager: AuthManager) -> Self {
        Self::new_with_network_policy(auth_manager, NetworkPolicy::default())
    }

    pub fn new_with_network_policy(
        auth_manager: AuthManager,
        network_policy: NetworkPolicy,
    ) -> Self {
        let http = network_policy
            .build_http_client()
            .expect("the fixed network policy should build an HTTP client");
        Self {
            auth_manager,
            http,
            network_policy,
            version: Arc::new(OnceCell::new()),
        }
    }

    pub fn network_policy(&self) -> NetworkPolicy {
        self.network_policy
    }

    /// The detected Docmost server version (from `POST /api/version`). Fetched once and
    /// cached **only on success**, so a transient probe failure doesn't poison the session
    /// (a later call re-probes). `None` if the endpoint is unavailable (e.g. Docmost Cloud)
    /// or the version is unparseable.
    pub async fn server_version(&self) -> Option<ServerVersion> {
        self.version
            .get_or_try_init(|| async {
                let response = self
                    .request::<VersionResponse>("/api/version", serde_json::json!({}), true)
                    .await
                    .map_err(|error| {
                        let _ = error;
                        debug_log::<serde_json::Value>(
                            "version",
                            "Could not determine Docmost version",
                            None,
                        );
                    })?;
                response.version().ok_or(())
            })
            .await
            .ok()
            .copied()
    }

    /// Version-gated server capabilities (see [`crate::version::Capabilities`]).
    pub async fn capabilities(&self) -> Capabilities {
        Capabilities::for_version(self.server_version().await)
    }

    pub async fn list_spaces(&self) -> Result<Vec<DocmostSpace>> {
        let result = self
            .request::<ListResult<DocmostSpace>>(
                "/api/spaces",
                serde_json::json!({ "page": 1, "limit": 100 }),
                true,
            )
            .await?;
        Ok(normalize_list_result(Some(result)))
    }

    pub async fn search_docs(
        &self,
        query: &str,
        space_id: Option<&str>,
    ) -> Result<Vec<DocmostSearchResult>> {
        validate_text("query", query, self.network_policy.max_search_bytes, false)?;
        validate_optional_text(
            "space_id",
            space_id,
            self.network_policy.max_identifier_bytes,
            false,
        )?;
        let mut payload = serde_json::json!({ "query": query });
        if let Some(space_id) = space_id {
            payload["spaceId"] = Value::String(space_id.to_string());
        }

        let result = self
            .request::<ListResult<DocmostSearchResult>>("/api/search", payload, true)
            .await?;
        Ok(normalize_list_result(Some(result)))
    }

    pub async fn get_space(&self, space_id: &str) -> Result<DocmostSpaceWithMembership> {
        validate_text(
            "space_id",
            space_id,
            self.network_policy.max_identifier_bytes,
            false,
        )?;
        self.request(
            "/api/spaces/info",
            serde_json::json!({ "spaceId": space_id }),
            true,
        )
        .await
    }

    pub async fn get_page(&self, slug_id: &str) -> Result<Option<DocmostPage>> {
        validate_text(
            "slug_id",
            slug_id,
            self.network_policy.max_identifier_bytes,
            false,
        )?;
        self.request(
            "/api/pages/info",
            serde_json::json!({ "pageId": slug_id }),
            true,
        )
        .await
    }

    pub async fn list_pages(
        &self,
        space_id: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<Vec<DocmostPageListItem>> {
        self.validate_page_request(space_id, limit, cursor)?;
        let mut payload = serde_json::json!({ "spaceId": space_id });
        if let Some(limit) = limit {
            payload["limit"] = Value::from(limit);
        }
        if let Some(cursor) = cursor {
            payload["cursor"] = Value::String(cursor.to_string());
        }

        let result = self
            .request::<CursorListResult<DocmostPageListItem>>("/api/pages/recent", payload, true)
            .await?;
        Ok(normalize_cursor_list_result(result))
    }

    pub async fn list_child_pages(
        &self,
        page_id: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<Vec<DocmostPageListItem>> {
        self.validate_page_request(page_id, limit, cursor)?;
        let mut payload = serde_json::json!({ "pageId": page_id });
        if let Some(limit) = limit {
            payload["limit"] = Value::from(limit);
        }
        if let Some(cursor) = cursor {
            payload["cursor"] = Value::String(cursor.to_string());
        }

        let result = self
            .request::<CursorListResult<DocmostPageListItem>>(
                "/api/pages/sidebar-pages",
                payload,
                true,
            )
            .await?;
        Ok(normalize_cursor_list_result(result))
    }

    pub async fn get_comments(
        &self,
        page_id: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<Vec<DocmostComment>> {
        self.validate_page_request(page_id, limit, cursor)?;
        let mut payload = serde_json::json!({ "pageId": page_id });
        if let Some(limit) = limit {
            payload["limit"] = Value::from(limit);
        }
        if let Some(cursor) = cursor {
            payload["cursor"] = Value::String(cursor.to_string());
        }

        let result = self
            .request::<CursorListResult<DocmostComment>>("/api/comments", payload, true)
            .await?;
        Ok(normalize_cursor_list_result(result))
    }

    pub async fn list_workspace_members(
        &self,
        limit: Option<u32>,
        cursor: Option<&str>,
        query: Option<&str>,
        admin_view: Option<bool>,
    ) -> Result<Vec<DocmostUser>> {
        validate_limit(limit, self.network_policy.max_list_limit)?;
        validate_optional_text(
            "cursor",
            cursor,
            self.network_policy.max_cursor_bytes,
            false,
        )?;
        validate_optional_text("query", query, self.network_policy.max_search_bytes, true)?;
        let mut payload = serde_json::json!({});
        if let Some(limit) = limit {
            payload["limit"] = Value::from(limit);
        }
        if let Some(cursor) = cursor {
            payload["cursor"] = Value::String(cursor.to_string());
        }
        if let Some(query) = query {
            payload["query"] = Value::String(query.to_string());
        }
        if let Some(admin_view) = admin_view {
            payload["adminView"] = Value::Bool(admin_view);
        }

        let result = self
            .request::<CursorListResult<DocmostUser>>("/api/workspace/members", payload, true)
            .await?;
        Ok(normalize_cursor_list_result(result))
    }

    pub async fn get_current_user(&self) -> Result<DocmostCurrentUserResponse> {
        self.request("/api/users/me", serde_json::json!({}), true)
            .await
    }

    fn validate_page_request(
        &self,
        id: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<()> {
        validate_text(
            "page or space ID",
            id,
            self.network_policy.max_identifier_bytes,
            false,
        )?;
        validate_limit(limit, self.network_policy.max_list_limit)?;
        validate_optional_text(
            "cursor",
            cursor,
            self.network_policy.max_cursor_bytes,
            false,
        )
    }

    /// POST with bearer auth and a single 401-retry, returning the raw response.
    async fn send_json(
        &self,
        endpoint: &str,
        payload: Value,
        retry_on_unauthorized: bool,
    ) -> Result<Response> {
        crate::network_policy::validate_json_size(
            "request body",
            &payload,
            self.network_policy.max_structured_content_bytes,
        )?;
        let request_bytes = serde_json::to_vec(&payload)?.len();
        let mut field_names = payload
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        field_names.sort();
        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let endpoint_class = endpoint.trim_start_matches("/api/");
        let mut session = self.auth_manager.get_authenticated_session().await?;
        let mut retry_on_unauthorized = retry_on_unauthorized;

        loop {
            debug_log(
                "api",
                "Calling Docmost API",
                Some(&serde_json::json!({
                    "endpointClass": endpoint_class,
                    "requestId": request_id,
                    "requestBytes": request_bytes,
                    "fieldNames": field_names,
                    "retryOnUnauthorized": retry_on_unauthorized
                })),
            );

            let response = self
                .http
                .post(format!("{}{}", session.base_url, endpoint))
                .bearer_auth(&session.token)
                .json(&payload)
                .send()
                .await
                .map_err(safe_transport_error)
                .with_context(|| format!("Failed to call {endpoint}"))?;

            debug_log(
                "api",
                "Docmost API response received",
                Some(&serde_json::json!({
                    "endpointClass": endpoint_class,
                    "requestId": request_id,
                    "status": response.status().as_u16(),
                    "ok": response.status().is_success()
                })),
            );

            if response.status() == reqwest::StatusCode::UNAUTHORIZED && retry_on_unauthorized {
                debug_log(
                    "api",
                    "Received 401 from Docmost API; retrying after reauthentication",
                    Some(&serde_json::json!({
                        "endpointClass": endpoint_class,
                        "requestId": request_id
                    })),
                );
                session = self.auth_manager.reauthenticate().await?;
                retry_on_unauthorized = false;
                continue;
            }

            return Ok(response);
        }
    }

    /// POST and deserialize the `{ data }` envelope, erroring if no data is returned.
    async fn request<T>(&self, endpoint: &str, payload: Value, retry: bool) -> Result<T>
    where
        T: DeserializeOwned,
    {
        parse_response(
            self.send_json(endpoint, payload, retry).await?,
            self.network_policy,
        )
        .await
    }

    /// POST a write that returns no meaningful body (e.g. move-to-space); succeeds on 2xx.
    async fn request_discard(&self, endpoint: &str, payload: Value) -> Result<()> {
        self.request_discard_with_retry(endpoint, payload, true)
            .await
    }

    /// POST a write whose response body is discarded, with explicit control over the
    /// single-401 replay used by ordinary writes. Destructive deletes pass `false`: an
    /// authorization refresh must never replay a deletion behind a stale confirmation.
    async fn request_discard_with_retry(
        &self,
        endpoint: &str,
        payload: Value,
        retry_on_unauthorized: bool,
    ) -> Result<()> {
        let response = self
            .send_json(endpoint, payload, retry_on_unauthorized)
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let details =
                safe_read_response_text(response, self.network_policy.max_error_body_bytes).await?;
            return Err(anyhow!(
                format!("Docmost API request failed ({status}). {details}")
                    .trim()
                    .to_string()
            ));
        }
        read_bounded_body(
            response,
            self.network_policy.max_success_body_bytes,
            "success response body",
        )
        .await?;
        Ok(())
    }
}

pub fn normalize_list_result<T>(result: Option<ListResult<T>>) -> Vec<T> {
    match result {
        Some(ListResult::List(items)) => items,
        Some(ListResult::Wrapped { items }) => items.unwrap_or_default(),
        None => Vec::new(),
    }
}

pub fn normalize_cursor_list_result<T>(result: CursorListResult<T>) -> Vec<T> {
    result.items.unwrap_or_default()
}

async fn parse_response<T>(response: Response, policy: NetworkPolicy) -> Result<T>
where
    T: DeserializeOwned,
{
    if !response.status().is_success() {
        let status = response.status();
        let details = safe_read_response_text(response, policy.max_error_body_bytes).await?;
        return Err(anyhow!(
            format!("Docmost API request failed ({status}). {details}")
                .trim()
                .to_string()
        ));
    }

    let body = read_bounded_body(
        response,
        policy.max_success_body_bytes,
        "success response body",
    )
    .await?;
    let json = serde_json::from_slice::<ApiEnvelope<T>>(&body)
        .context("Failed to parse Docmost API response body")?;
    json.data
        .ok_or_else(|| anyhow!("Docmost API response was missing a data payload"))
}
