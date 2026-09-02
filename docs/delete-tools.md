# Destructive delete tools

These tools are absent from the default read-only inventory. Each must be named
individually in the exact write allowlist, and Atlas confirmation remains
mandatory. All three annotations are `readOnlyHint: false`,
`destructiveHint: true`, and `idempotentHint: false`.

## Docmost Community v0.95.0 contracts

| Tool | Required stable target | Endpoint and payload | Consequence |
| --- | --- | --- | --- |
| `delete_page` | `page_id` UUID | `POST /api/pages/delete`, `{pageId, permanentlyDelete: false}` | Moves the target and all active descendants to trash and removes active page shares. It does not permanently purge the pages. |
| `delete_space` | `space_id` UUID | `POST /api/spaces/delete`, `{spaceId}` | Permanently deletes the space; database foreign keys cascade space-owned pages, comments, memberships, shares, and related rows. Docmost queues attachment cleanup after deleting the space. |
| `delete_comment` | `comment_id` UUID | `POST /api/comments/delete`, `{commentId}` | Permanently deletes the comment; the comment parent foreign key cascade-deletes threaded replies. |

These contracts are pinned to the Docmost `v0.95.0` source tag at commit
`4132dd597c956a27423607d008708c0e214690da` (tree
`2da55313c0d99912764eebf82a138c305271420d`).

## Retry and failure behavior

The HTTP client keeps the existing five-second connect timeout, thirty-second
request timeout, no-redirect policy, bounded input/output bodies, bearer
authentication, and sanitized diagnostics. Deletes deliberately opt out of the
ordinary single-401 replay. They make one network dispatch per tool call.

Any non-2xx status, redirect, timeout, oversized response, connection failure,
or interrupted call fails without an automatic retry. The error says that the
deletion was not confirmed and requires independent target inspection before a
new confirmation. This distinction matters: a timeout may occur after Docmost
commits a delete, so treating it as proof of absence and retrying would be unsafe.

An unknown or already absent target therefore fails rather than being treated as
idempotent success. A duplicate explicit invocation can produce one successful
delete followed by a `404`; the client never turns that sequence into a second
delete.

## Success result

Successful calls return sanitized JSON with this stable shape:

```json
{
  "outcome": "moved_to_trash or permanently_deleted",
  "target": { "type": "page, space, or comment", "id": "target UUID" },
  "consequence": { "operation-specific": "cascade context" },
  "automaticRetry": false
}
```

The output includes only the caller-supplied canonical UUID and fixed,
code-owned consequence labels; it does not echo response bodies or remote
content.
