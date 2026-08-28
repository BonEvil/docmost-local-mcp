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

This repository has no advisory exceptions. `deny.toml` has an empty
`[advisories].ignore` list; any RustSec advisory, including high and critical
severity advisories, fails CI through `cargo-deny`. An exception may not be
added merely because a dependency is optional: it must identify the exact
feature and target, include a reproducible `cargo tree --locked --target …
--features …` proof that the package is absent, and be removed once the path
is supported again. The policy's current empty exception set needs no such
reachability evidence.

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
