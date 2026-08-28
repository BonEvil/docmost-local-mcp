# Dependency policy

## Supported graph

The supported release graph is browser-authentication only. The former
`native-webview` Cargo feature remains a no-op compatibility switch, so both
`--no-default-features` and `--all-features` resolve the same Rust graph. The
embedded GTK/WebKit/native-webview chain (`tao`, `wry`, GTK3, and WebKitGTK) is
not in `Cargo.lock` and is not a supported feature or platform path.

CI checks these supported configurations with a locked lockfile:

| Runner | Feature command | Required checks |
| --- | --- | --- |
| Ubuntu 24.04 | `--no-default-features` | fmt, Clippy, tests, dependency policy |
| macOS 15 | `--all-features` | Clippy, tests |
| Windows 2025 | `--all-features` | Clippy, tests |

Release platform builds use `cargo build --locked --release --no-default-features`.

## Advisory disposition

All high- and critical-severity advisories fail CI through `cargo-deny`. The
single narrow RustSec exception in `deny.toml` is reviewed, not a blanket
suppression:

| Advisory | Exact path | Reproducible reachability evidence | Disposition |
| --- | --- | --- | --- |
| RUSTSEC-2026-0189 (rmcp Streamable HTTP Host validation) | `docmost-local-mcp` → `rmcp 0.6.4`, enabled only through `transport-io`/`transport-async-rw` | `cargo tree --locked -e features -i rmcp` shows `transport-io`; source imports only `rmcp::transport::io::stdio` in `src/main.rs`; no rmcp Streamable HTTP listener is constructed. The binary's only local HTTP service is the separate Axum auth handler bound to loopback. | The advisory explicitly states stdio and child-process transports are unaffected. The affected Streamable HTTP path is unreachable in every supported build. Revisit on each rmcp update. |

`RUSTSEC-2024-0436` for transitive `paste` is not an advisory exception: it is
an informational unmaintained notice with no CVE, CVSS score, or security
impact. `unmaintained = "workspace"` keeps the notice visible while causing
CI to fail if an unmaintained crate becomes a direct workspace dependency.

Any new exception must identify its exact feature and target, include the
corresponding locked dependency-tree and source-reachability evidence, and be
removed when the path becomes supported or an upstream fix is available.

The lockfile refresh on 2026-08-28 removes the native GTK/WebKit chain and
updates the remaining active graph, including `anyhow` 1.0.104,
`rustls-webpki` 0.103.15, `rmcp` 0.6.4, `quinn-proto` 0.11.17, and `rand`
0.10.2. `time` is absent from the retained lockfile.

## Reproducibility and maintenance

Lockfile identity after this refresh:

```text
Cargo.lock SHA-256: 1e0246190172c39b9adf7597c6abbae9771fe00e88879f38e4bdabb4c45fe07f
Generator: cargo 1.98.0
Refresh command: cargo update
```

Run the same checks locally with a Rust toolchain:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo deny check advisories bans licenses sources
```

Dependabot opens weekly Cargo, GitHub Actions, and launcher npm dependency
updates. The scheduled CI run repeats the policy checks every Monday; it does
not use repository secrets.
