//! Explicitly destructive delete tools. Their descriptions and input schemas carry
//! target/consequence context for Atlas confirmation before dispatch. Success results are
//! machine-readable JSON; failures are conservative and never replayed automatically.

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::ErrorData,
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    server::{DocmostMcpServer, internal_error},
    types::{DeleteCommentInput, DeletePageInput, DeleteSpaceInput},
};

#[tool_router(router = delete_tool_router, vis = "pub(crate)")]
impl DocmostMcpServer {
    #[tool(
        name = "delete_page",
        description = "Move one Docmost page and all active descendant pages to trash; active page shares are removed. Requires the stable page UUID. This is destructive but not a permanent purge.",
        annotations(
            title = "Delete Docmost Page and Descendants",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn delete_page(
        &self,
        Parameters(input): Parameters<DeletePageInput>,
    ) -> Result<Json<DeleteToolResult>, ErrorData> {
        self.client
            .delete_page(&input.page_id)
            .await
            .map_err(internal_error)?;
        Ok(delete_result(
            "page",
            &input.page_id,
            "moved_to_trash",
            vec!["active_descendant_pages", "active_page_shares"],
            false,
            None,
        ))
    }

    #[tool(
        name = "delete_space",
        description = "Permanently delete one Docmost space. All space-owned pages, comments, memberships, shares, and related records are cascade-deleted; attachment cleanup is queued by Docmost. Requires the stable space UUID.",
        annotations(
            title = "Permanently Delete Docmost Space and Contents",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn delete_space(
        &self,
        Parameters(input): Parameters<DeleteSpaceInput>,
    ) -> Result<Json<DeleteToolResult>, ErrorData> {
        self.client
            .delete_space(&input.space_id)
            .await
            .map_err(internal_error)?;
        Ok(delete_result(
            "space",
            &input.space_id,
            "permanently_deleted",
            vec![
                "space_owned_pages",
                "comments",
                "memberships",
                "shares",
                "related_space_records",
            ],
            true,
            Some("attachment_cleanup_queued_by_docmost"),
        ))
    }

    #[tool(
        name = "delete_comment",
        description = "Permanently delete one Docmost comment. Threaded replies below it are cascade-deleted. Requires the stable comment UUID.",
        annotations(
            title = "Permanently Delete Docmost Comment and Replies",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn delete_comment(
        &self,
        Parameters(input): Parameters<DeleteCommentInput>,
    ) -> Result<Json<DeleteToolResult>, ErrorData> {
        self.client
            .delete_comment(&input.comment_id)
            .await
            .map_err(internal_error)?;
        Ok(delete_result(
            "comment",
            &input.comment_id,
            "permanently_deleted",
            vec!["threaded_replies"],
            true,
            None,
        ))
    }
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteToolResult {
    outcome: String,
    target: DeleteTarget,
    consequence: DeleteConsequence,
    automatic_retry: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeleteTarget {
    #[serde(rename = "type")]
    target_type: String,
    id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteConsequence {
    cascade: Vec<String>,
    permanent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    asynchronous_follow_up: Option<String>,
}

fn delete_result(
    target_type: &str,
    target_id: &str,
    outcome: &str,
    cascade: Vec<&str>,
    permanent: bool,
    asynchronous_follow_up: Option<&str>,
) -> Json<DeleteToolResult> {
    Json(DeleteToolResult {
        outcome: outcome.to_string(),
        target: DeleteTarget {
            target_type: target_type.to_string(),
            id: target_id.to_string(),
        },
        consequence: DeleteConsequence {
            cascade: cascade.into_iter().map(str::to_string).collect(),
            permanent,
            asynchronous_follow_up: asynchronous_follow_up.map(str::to_string),
        },
        automatic_retry: false,
    })
}
