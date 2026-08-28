# Contributing

Thanks for contributing to the BonEvil `docmost-local-mcp` hardened fork.

This document covers local setup and repository checks. Security-preserving
sync, patch, advisory, release, and Atlas procedures are in
[`docs/operations-and-maintenance.md`](docs/operations-and-maintenance.md).

## Prerequisites

- Node.js 20 for legacy launcher syntax and smoke validation
- Rust toolchain (`cargo`, `rustc`)
- A reachable Docmost instance for manual auth testing

Authentication uses the system browser on every platform. Native embedded
webviews are intentionally unsupported because their GTK/WebKit dependency
chain could not meet this project's active dependency policy.

## Local Setup

```bash
cargo build
```

Useful commands:

```bash
cargo fmt
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features
cargo build --release
```

The former `native-webview` feature is a no-op compatibility switch; all builds
use the browser-authentication flow:

```bash
cargo build --release --no-default-features
```

## Repository Layout

- `src/`: Rust MCP server, auth flow, Docmost client, storage, and ProseMirror conversion
- `npm/launcher/`: retained legacy launcher code; not an Atlas production path
- `.github/workflows/`: CI and release workflows

## Local npx-style test (launcher + binary)

To verify the full path (Node launcher → binary → MCP server) without publishing:

1. Build the release binary: `cargo build --release`
2. Place it where the launcher expects:

   ```bash
   mkdir -p npm/launcher/bin
   cp target/release/docmost-local-mcp npm/launcher/bin/
   ```

   On Windows, copy `docmost-local-mcp.exe` instead.

3. Run the launcher:

   ```bash
   node npm/launcher/cli.js --help
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | node npm/launcher/cli.js --base-url=https://example.com
   ```

## Local MCP Testing

Run the binary directly:

```bash
cargo run -- --base-url=https://docs.example.com
```

For MCP client configuration from a local checkout:

```json
{
  "mcpServers": {
    "docmost": {
      "command": "/absolute/path/to/docmost-local-mcp/target/debug/docmost-local-mcp",
      "args": ["--base-url=https://docs.example.com"]
    }
  }
}
```

## From-Scratch Auth Testing

To test first-time auth without touching your real saved state, run the MCP server with a temporary `HOME`:

```bash
TMP_HOME="$(mktemp -d /tmp/docmost-local-mcp-test.XXXXXX)"
HOME="$TMP_HOME" cargo run -- --base-url=https://docs.example.com
```

This forces the package to create a fresh `~/.docmost-local-mcp/` under the temporary home directory.

## CI

`ci.yml` runs:

- `cargo fmt --check`
- locked Clippy and tests for Linux headless, macOS compatibility, and Windows compatibility configurations
- `cargo deny check advisories bans licenses sources` with only the exact,
  documented unreachable-path exception in `deny.toml`
- weekly Dependabot updates and a scheduled weekly CI policy run
- MCP tool registration coverage, including object-shaped input schemas for no-arg tools
- a launcher smoke test with a mock binary in `bin/`
- release binary builds on:
  - `macos-15`
  - `macos-15-intel`
  - `ubuntu-24.04-arm`
  - `ubuntu-24.04`
  - `windows-11-arm`
  - `windows-2025`

## Release model

Atlas production does not use the npm launcher or downloader. The fork-owned
release workflow builds versioned platform binaries, a commit-bound manifest,
checksums, a Sigstore bundle, and GitHub provenance. Publication is a separately
approved maintainer action after every repository and live gate passes. See
[`docs/atlas-release-integrity.md`](docs/atlas-release-integrity.md).
