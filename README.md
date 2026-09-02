# docmost-local-mcp hardened fork

[![license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

MCP server for [Docmost](https://docmost.com/) that is built for self-hosted instances, especially deployments that do not have an enterprise license but still want reliable MCP access from local IDEs and AI tools.

Atlas production launches a reviewed Rust binary from this fork by absolute path.
The supported installer verifies a reviewed SHA-256 digest and a keyless-signed,
commit-bound release manifest before an atomic install. The legacy npm downloader
is disabled. See [Atlas release integrity](docs/atlas-release-integrity.md).

> The main reason this project exists: bring MCP access to self-hosted Docmost setups without making an enterprise license a prerequisite.

## Why This Project

Many MCP integrations are designed around hosted or enterprise assumptions. This project is intentionally optimized for self-hosted Docmost:

- Works against your own Docmost base URL
- Uses Docmost email/password authentication
- Stores session state locally for reuse
- Opens a local auth flow instead of requiring a separate hosted control plane
- Ships as a versioned, checksum- and provenance-verified Rust binary

If you run your own Docmost and want it available inside Cursor, Claude Desktop, or another MCP client, this package is the straightforward path.

## Highlights

- Strong fit for self-hosted Docmost instances without enterprise licensing
- Rust server core distributed as provenance-verified platform binaries
- System-browser authentication on every supported platform
- Explicit Docmost instance selection via startup config
- Session reuse with JWT expiry checks and automatic re-login
- OS keychain credential storage on supported platforms
- Clean tool surface for spaces, pages, comments, members, and current user context

## Available Tools

The server starts in **read-only mode**. The default inventory is the ten read
tools below; no persistent mutation is registered:

- `list_spaces`: list available Docmost spaces
- `get_space`: fetch details for a specific space
- `search_docs`: search documentation, optionally scoped to a space
- `search_pages`: backward-compatible alias for `search_docs`
- `get_page`: fetch a page and return its content as Markdown
- `list_pages`: list recent pages in a space
- `list_child_pages`: list child pages for a parent page ID
- `get_comments`: list comments for a page
- `list_workspace_members`: list workspace members
- `get_current_user`: fetch the authenticated user and workspace context

The following write tools are unavailable unless the operator enables write
mode and names each tool in the allowlist:

- `create_page`: create a new page in a space from Markdown content
- `update_page`: update an existing page's title and/or Markdown content
- `duplicate_page`: duplicate a page (and its sub-pages) within its space
- `copy_page_to_space`: copy a page (and its sub-pages) into a different space
- `move_page`: move a page under a new parent page, or to the space root
- `move_page_to_space`: move a page (and its sub-pages) to a different space
- `create_space`: create a new space with a name and URL slug
- `update_space`: update a space's name, slug, and/or description
- `create_comment`: add a page-level comment to a page from Markdown
- `update_comment`: replace an existing comment's body with new Markdown
- `delete_page`: move a page and all active descendants to trash
- `delete_space`: permanently delete a space and its space-owned content
- `delete_comment`: permanently delete a comment and threaded replies

See [Authority modes](docs/authority-modes.md) for the fail-closed configuration,
the exact inventories and annotations, and the independent Atlas confirmation
requirement.

Authenticated network calls use fixed connect/request deadlines, bounded request and
response bodies, a no-redirect policy, and content-free structured diagnostics. See
[Network and diagnostics safety](docs/network-and-diagnostics-safety.md) for the complete
limit and redirect decision tables.

## Roadmap

All planned read and write tools are now implemented. `create_comment` adds
page-level comments; comments anchored to a specific text selection (inline
comments) require the collaborative editor's cursor positions and are out of
scope for this REST-based server.

## Compatibility

Compatibility with Docmost Community **v0.95.0** passed only in a bounded,
isolated disposable environment; it is not evidence of, and does not authorize,
ordinary production mutation. See the sanitized
[compatibility report](verification/docmost-v0.95.0-compatibility-report.md).
Deployment remains separately disabled until the required release gates in
[Operations and maintenance](docs/operations-and-maintenance.md) pass for the
exact candidate. The server detects the Docmost version (via
`POST /api/version`) once per session and adapts where behaviour differs:

- **Page body edits:** `update_page` can only change an existing page's **body**
  on Docmost **v0.70.0+**. On older servers the body lives in the collaborative
  editor and a REST body update is ignored — `update_page` says so explicitly and
  suggests `create_page` (which persists bodies on every version via the import
  endpoint). Title updates work everywhere.

## Content support

Page and comment bodies are written in Markdown (headings, bold/italic/strike,
inline code, links, lists, task lists, blockquotes, code blocks; pages also
support tables and external-URL images). Comments support a smaller set — no
tables, task lists, or images.

**Mentions (@tagging):** in any Markdown body, a link with a `user:` or `page:`
URL becomes a mention — `[Display Name](user:USER_UUID)` tags a user (find the
UUID with `list_workspace_members`) and `[Page Title](page:PAGE_UUID)` links a
page. Tagged users are notified by Docmost.

Attaching uploaded files/images (as opposed to referencing an image URL) is not
supported.

## Requirements

- A binary installed through the verified fork procedure
- A reachable Docmost instance
- Email/password authentication enabled in that Docmost instance

## Quick Start

After completing the reviewed installation procedure, run the pinned binary:

```bash
/opt/atlas/mcp/docmost-local-mcp --base-url=https://docs.example.com
```

This command is read-only. A narrowly scoped write launch must include both
explicit write mode and a nonempty allowlist, for example:

```bash
/opt/atlas/mcp/docmost-local-mcp \
  --base-url=https://docs.example.com \
  --authority-mode=write \
  --write-tools=create_page,update_page
```

Environment equivalents are `DOCMOST_AUTHORITY_MODE=write` and
`DOCMOST_WRITE_TOOLS=create_page,update_page`.

You can also provide the base URL with an environment variable:

```bash
DOCMOST_BASE_URL=https://docs.example.com /opt/atlas/mcp/docmost-local-mcp
```

## MCP Client Configuration

Configure the client with the absolute verified-binary path, replacing the base URL with your own Docmost instance:

```json
{
  "mcpServers": {
    "docmost": {
      "command": "/opt/atlas/mcp/docmost-local-mcp",
      "args": ["--base-url=https://docs.example.com"]
    }
  }
}
```

Where that config lives, per client:

- **Claude Desktop** — `claude_desktop_config.json` (Settings → Developer → Edit Config)
- **Cursor** — `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (per project)
- **Claude Code** — one command, no file editing:

  ```bash
  claude mcp add docmost -- /opt/atlas/mcp/docmost-local-mcp --base-url=https://docs.example.com
  ```

- **VS Code (GitHub Copilot)** — `.vscode/mcp.json`, using a top-level `servers` key instead of `mcpServers`:

  ```json
  {
    "servers": {
      "docmost": {
        "command": "/opt/atlas/mcp/docmost-local-mcp",
        "args": ["--base-url=https://docs.example.com"]
      }
    }
  }
  ```

This setup works well when you want a fixed Docmost instance per client configuration. If `--base-url` or `DOCMOST_BASE_URL` is set, the login page shows that URL prefilled and locks the field. If no base URL is configured, the login flow asks for it during interactive sign-in.

## Example Prompts

Once connected, ask your AI client things like:

- "Search the Engineering space in Docmost for our on-call runbook and summarize it."
- "Create a new page in the Product space titled 'Q3 Planning' from these notes: …"
- "Turn this meeting transcript into a structured Docmost page under the Team space."
- "Find every page in Docmost that mentions the old API endpoint and list them."
- "Read the 'Onboarding' page and draft a shorter checklist as a new sub-page."
- "Add a comment on the release-notes page flagging the missing migration step."

## Authentication Flow

1. Your MCP client launches the server over stdio. Initialize-first clients use the standard MCP handshake unchanged. Atlas MCP 2.0 may first send `server/discover`; the server returns the legacy JSON-RPC fallback and keeps stdio open so Atlas can initialize and enumerate the same inventory. This bounded preflight runs before credential state or the Docmost client is opened.
2. On the first authenticated tool call, the server starts a local HTTP login page on `127.0.0.1`.
3. The server opens the system browser for the loopback authentication flow.
4. You enter your email and password there. If `--base-url` or `DOCMOST_BASE_URL` is set, the Docmost URL is prefilled and locked.
5. The server signs in through `/api/auth/login`, extracts the `authToken` cookie, stores the session, and optionally stores credentials for automatic re-login.
6. Future requests reuse the saved token until it is close to expiry or rejected by Docmost.

## Local State And Credential Storage

The server stores local state in:

```text
~/.docmost-local-mcp/
```

Files used there:

- `config.json`: last base URL and email
- `session.json`: saved auth token and expiry

Credentials are stored in the OS keychain when available, which is the preferred path on supported platforms.

Passwords are not persisted by default. Interactive login stores only the resulting origin-bound session unless the operator selects **Remember password**. Remembered passwords use secure OS credential storage; keyring failures fail closed.

The weaker encrypted-file fallback is disabled by default. To acknowledge that its encryption key is stored in the same protected local directory as its ciphertext, explicitly start the server with `--allow-insecure-credential-file` or `DOCMOST_ALLOW_INSECURE_CREDENTIAL_FILE=true`. This never bypasses the keyring when the keyring works.

Remove authentication state for one canonical origin with:

```bash
docmost-local-mcp forget --base-url https://docs.example.com
```

For deliberately enabled literal-loopback Docmost HTTP, add `--allow-insecure-loopback-http` to the `forget` subcommand. Forget is idempotent and removes the origin's keyring entry, session, fallback ciphertext/key, matching config, legacy unscoped keyring entry, and stale/unscoped legacy files without deleting another origin's scoped state. See [Credential and loopback authentication lifecycle](docs/credential-auth-lifecycle.md).

## Platform Notes

Authentication always uses the system browser. The retained `native-webview`
Cargo feature is a no-op compatibility switch; embedded webview dependencies
are absent from the supported dependency graph.

## Tool Reference

### `list_spaces`

Returns Docmost space names, slugs, and IDs.

### `search_docs`

Inputs:

- `query`: required search text
- `space_id`: optional Docmost space ID

### `search_pages`

Inputs:

- `query`: required search text
- `space_id`: optional Docmost space ID

This is a backward-compatible alias for page search. `search_docs` remains available.

### `get_space`

Inputs:

- `space_id`: required Docmost space ID

### `get_page`

Inputs:

- `slug_id`: the page slug ID returned by `search_docs`

### `list_pages`

Inputs:

- `space_id`: required Docmost space ID
- `limit`: optional page count limit
- `cursor`: optional pagination cursor

### `list_child_pages`

Inputs:

- `page_id`: required parent page ID
- `limit`: optional page count limit
- `cursor`: optional pagination cursor

### `get_comments`

Inputs:

- `page_id`: required page ID
- `limit`: optional comment count limit
- `cursor`: optional pagination cursor

### `list_workspace_members`

Inputs:

- `limit`: optional member count limit
- `cursor`: optional pagination cursor
- `query`: optional member search text
- `admin_view`: optional admin visibility flag

### `get_current_user`

Inputs:

- none

### `create_page`

Inputs:

- `space_id`: required Docmost space ID (UUID) to create the page in
- `title`: required page title
- `markdown`: optional page body as Markdown
- `parent_page_id`: optional parent page ID to nest under (title-only pages only)

When `markdown` is provided, the page body is sent through Docmost's **import** endpoint
(`POST /api/pages/import`), which is the only mechanism that reliably persists page body
content across Docmost versions (including older self-hosted servers). Pages created with
a body land at the space root — `parent_page_id` is honored only for title-only pages.

### `update_page`

Inputs:

- `page_id`: required Docmost page ID or slug ID
- `title`: optional new title (omit to leave unchanged)
- `markdown`: optional new body as Markdown; replaces the existing content (omit to leave unchanged)

Updating a page **title** works on all Docmost versions. Updating an existing page's
**body** via REST works only on newer Docmost; on older self-hosted servers (e.g. v0.25.x)
the body is edited solely through the collaborative editor and a REST body update is not
applied. To set body content reliably there, create a new page with `create_page` instead.

For the full design, Markdown→ProseMirror conversion details, verified Docmost API
fields, and version caveats, see [docs/write-tools.md](docs/write-tools.md).

### Destructive deletes

`delete_page`, `delete_space`, and `delete_comment` each require the target's
stable UUID. Their tool metadata states the cascade consequence before dispatch,
and each successful result is sanitized JSON containing `outcome`, `target`,
`consequence`, and `automaticRetry: false`. Delete requests are never retried
automatically, including after HTTP 401: an interrupted or timed-out request can
have committed remotely, so inspect the target before authorizing another call.
See [Destructive delete tools](docs/delete-tools.md).

## Development

For maintainer and contributor workflow details, see `CONTRIBUTING.md`.

## License

MIT
