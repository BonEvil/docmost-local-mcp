# CLAUDE.md

Guidance for working in this repository.

## Response style

- **Explain simply.** Answer in plain, concise language. Avoid jargon where a plain word works; when a technical term is unavoidable, define it in one short clause.
 
- **Lead with the diagram or a one-line summary**, then add brief supporting text — not a wall of prose.

## What this is

`docmost-local-mcp` is a hardened **Rust MCP server** that fronts a self-hosted [Docmost](https://docmost.com) instance for local IDE / AI tools. It speaks MCP over **stdio** using [`rmcp`](https://docs.rs/rmcp) 0.6, authenticates to Docmost with origin-scoped local state, exposes ten reads by default, and permits writes only through an explicit narrow allowlist. Atlas production uses the reviewed `BonEvil/docmost-local-mcp` binary and never the upstream npm downloader.

Edition **2024**. The crate is both a binary and a library (`docmost_local_mcp`); integration tests in `tests/` consume the library.

## Commands

```bash
cargo build                 # debug build (browser-authentication flow)
cargo build --release       # release build
cargo test                  # unit + integration tests (tests/ use tempfile + mock axum servers)
cargo fmt --check                                        # format — CI runs this
cargo clippy --locked --all-targets --all-features -- -D warnings  # lint — CI fails on any warning

# Run the MCP server locally (talks MCP over stdio):
cargo run -- --base-url=https://docs.example.com
DOCMOST_BASE_URL=https://docs.example.com cargo run

# Compatibility feature: native-webview is intentionally a no-op; all builds use browser login.
cargo build --no-default-features
```

- `--base-url` (or `DOCMOST_BASE_URL`) selects the Docmost instance; it's optional — if absent the interactive login asks for it.
- `DOCMOST_DISABLE_KEYRING=1` skips the OS keyring and uses the encrypted-file credential fallback (used by tests).
- `DEBUG_DOCMOST_MCP=1` (or `true`) enables debug logging (via `debug::debug_log`, prefix `[docmost-local-mcp][ts][scope]`), which goes to **stderr** — never stdout, which is reserved for the MCP protocol.

## Architecture

Request path: **MCP client → `DocmostMcpServer` (`#[tool_router]`) → `DocmostClient` → Docmost `/api/...` → Markdown string back to the client.**

- [src/main.rs](src/main.rs) — `clap` CLI. Default command builds `StartupConfig` and serves over `stdio()`. The hidden `auth-window` command remains only for compatibility and reports that embedded windows are unsupported.
- [src/server.rs](src/server.rs) + [src/server/tools.rs](src/server/tools.rs) + [src/server/tools_write.rs](src/server/tools_write.rs) + [src/server/tools_delete.rs](src/server/tools_delete.rs) — `DocmostMcpServer { client, tool_router }`, `#[tool_handler] impl ServerHandler`. **23 tools**: 10 read-only plus 13 exact allowlistable writes. The delete router contains `delete_page`, `delete_space`, and `delete_comment`; all are destructive, non-idempotent, and non-read-only.
- [src/server/render.rs](src/server/render.rs) — formats domain structs into the **Markdown** strings tools return (results are truncated: search 5, lists 10, members 20).
- [src/prosemirror/](src/prosemirror/) — Markdown ⇄ ProseMirror conversion, split into `reader.rs` (JSON → Markdown), `writer/` (Markdown → JSON, event-walker in `writer/build.rs`), and `nodes.rs` (node builders). Mentions use a link convention: `[label](user:UUID)` / `[label](page:UUID)` → a `mention` node (inline atom with `entityType`/`entityId`; each gets a unique `id` since Docmost dedups by it). Comments accept a StarterKit subset — no tables/task-lists/images.
- [src/docmost_client.rs](src/docmost_client.rs) — `reqwest` wrapper. Every call is `POST {base_url}{endpoint}` with `bearer_auth(token)`; responses are unwrapped from an `{ "data": ... }` envelope. List shapes normalized by `normalize_list_result` / `normalize_cursor_list_result`. **Retries once on HTTP 401** after reauthenticating.
- [src/auth/](src/auth/) — `manager.rs` (session lifecycle: reuse saved session unless within 2 min of JWT expiry, else reauth via saved credentials or interactive login), `local_server.rs` (axum login page on `127.0.0.1:<random>`), `webview.rs` (system-browser authentication; no embedded webview dependencies).
- [src/storage/](src/storage/) — `state_store.rs` persists to `~/.docmost-local-mcp/` (`config.json`, `session.json`); credentials go to the **OS keyring first** (`keyring_store.rs`), falling back to an **AES-256-GCM** encrypted file. Writes are atomic (temp + rename) with `0o600` perms.
- [src/types/](src/types/) — `mod.rs` holds the serde domain models (`#[serde(rename_all = "camelCase")]`); `inputs.rs` holds the `JsonSchema` tool-input structs (re-exported, so `crate::types::*Input` paths are unchanged).
- [src/version.rs](src/version.rs) — `ServerVersion` (parsed from `POST /api/version`, cached once on the client) + version-gated `Capabilities`. Unknown version ⇒ conservative (no capability claimed). Supported floor ≈ **v0.22** (~1 year); REST page-body update gated at **v0.70.0**.
- [npm/launcher/](npm/launcher/) — the Node `npx` launcher (`cli.js`) + `postinstall.js` that downloads the platform binary from GitHub Releases. CI (`.github/workflows/ci.yml`) runs rust checks, a launcher smoke test, and release builds for 6 platforms.

## Conventions & gotchas

- **Page `position` keys.** `move_page` appends a page after its target parent's last child using [src/position/](src/position/) — a faithful Rust port of the base62 `fractional-indexing-jittered` scheme Docmost uses (validated 5..=12 chars). The port is checked against the upstream package's own reference vectors.
- **Read + thirteen write tools.** The exact write allowlist adds `delete_page`, `delete_space`, and `delete_comment` to the prior ten. Delete requests require canonical UUIDs and disable automatic 401 replay; their successful results are sanitized structured JSON with target, consequence, outcome, and retry policy.
- Read and ordinary write tools return human-readable Markdown. Delete tools intentionally return sanitized structured JSON for destructive confirmation/audit context.
- Tool args are `schemars::JsonSchema` structs in `types.rs`, passed as `Parameters<T>`; required fields are non-`Option`, optional fields use `#[serde(default)] Option<T>`.
- Auth is **lazy** — it triggers on the first authenticated tool call, not at startup. Never log tokens, passwords, or cookies to stdout.
- The Docmost API wraps payloads in `{ "data": ... }` and returns lists either bare or under `items`; use the existing normalize helpers.
- Adding a tool means touching three places: input struct in `types.rs`, `#[tool]` method in `server/tools.rs`, client method (+ endpoint) in `docmost_client.rs`, and usually a formatter in `render.rs`. Mirror an existing tool and add a check to `tests/mcp_server_test.rs` (which asserts the expected tool list and schemas).

See `CONTRIBUTING.md` for maintainer/release workflow.
