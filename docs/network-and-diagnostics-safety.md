# Network and diagnostics safety

Authentication and authenticated API calls share a fail-closed network policy. The
production values are compiled into the reviewed binary and are not environment or CLI
tunables. This prevents an Atlas launch configuration from accidentally removing a
deadline or weakening a content bound.

## Configured limits

| Control | Production default | Applies to | Failure behavior |
| --- | ---: | --- | --- |
| Connect deadline | 5 seconds | Login and API connections | Request fails; no retry except the existing single 401 reauthentication path |
| Overall request deadline | 30 seconds | Headers and streamed body for every login and API attempt | Request fails deterministically |
| Success response body | 8 MiB | JSON API success envelopes | Body is rejected before parsing |
| Error response body | 64 KiB | Login and API non-2xx responses | Body is rejected when oversized; otherwise consumed and omitted from the error |
| Authentication success body | 64 KiB | Login response after the cookie header is found | Login fails before session state is persisted |
| Markdown | 4 MiB | Imported page bodies | Input is rejected before the request |
| MCP page Markdown output | 4 MiB | Rendered `get_page` result including metadata | Output fails closed before it is returned |
| Structured page/comment content | 4 MiB serialized JSON | Update bodies and comments | Input is rejected before the request |
| Search/member query | 4 KiB UTF-8 | Search text and member filter | Input is rejected before the request |
| Identifier | 4 KiB UTF-8 | Page, space, comment, and slug IDs | Input is rejected before the request |
| Cursor | 4 KiB UTF-8 | Pagination cursors | Input is rejected before the request |
| Title/name | 1 KiB UTF-8 | Page titles and space names | Input is rejected before the request |
| Description | 16 KiB UTF-8 | Space descriptions | Input is rejected before the request |
| List limit | 1 through 100 | Page, child-page, comment, and member lists | Values outside the inclusive range are rejected before the request |

Body enforcement checks a declared `Content-Length` first and also counts each streamed
chunk. The streaming count is authoritative when a server omits or lies about the header.
All sizes are UTF-8 byte counts, not character counts.

## Redirect decisions

The canonical origin is selected and pinned by authentication. The HTTP client uses a
no-redirect policy for both login and authenticated API operations.

| Response | Decision | Credentials sent to redirect target? |
| --- | --- | --- |
| 2xx from canonical origin | Process under the applicable body cap | Not applicable |
| 401 from canonical origin, first attempt | Reauthenticate, then retry the same canonical endpoint once | No |
| 401 after retry | Return the status as an error | No |
| 3xx to the same origin | Return the 3xx as an error | No |
| 3xx to a different origin, scheme, or port | Return the 3xx as an error | No |
| Redirect chain or loop | The first 3xx terminates the operation | No |

Refusing same-origin redirects as well as cross-origin redirects keeps the decision simple,
auditable, and independent of library-specific header-forwarding behavior.

## Safe diagnostics

`DEBUG_DOCMOST_MCP=1` enables structured diagnostics, but the detail serializer uses a
positive allowlist. It emits only reviewed metadata such as endpoint class, process-local
request ID, request/response byte counts, HTTP status, retry state, booleans, and approved
top-level field names. Unknown fields are omitted rather than serialized.

Normal and debug diagnostics never include passwords, bearer tokens, cookies, email
addresses, request values, search text, page bodies, Markdown, comments, response
bodies, or server error excerpts. For a bounded nonempty error body, the caller receives
only `Response body omitted (N bytes).`; oversized error bodies produce the applicable
limit failure. There is no unsafe-content logging mode.
