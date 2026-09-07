# Attachment and rich-media verification

Implementation commit: `f8018b515e960767ea8f6288d247cb29c75b041f`.
Base: `5f87ca6f264e96baba5a4981ebb098cc9f77a300`.
Verification performed in an isolated Rust 1.98.0 Linux container and a fresh
Docmost Community v0.95.0 stack. No production content or authentication state
was modified. No release, merge, binary installation, or connector reconfiguration
was performed.

## Results

- `cargo check --locked`: passed.
- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked --all-features`: 168 passed, 2 opt-in live tests ignored.
- `cargo test --locked --no-default-features`: 168 passed, 2 opt-in live tests ignored.
- Opt-in `media_live_test` against a fresh disposable v0.95.0: 1 passed.
- `tests/release_integrity_test.sh`, `tests/release_context_test.sh`, and
  `tests/version_consistency_test.sh`: passed.
- `git diff --check`: passed.

The final Clippy-only correction removed a redundant struct-default initializer
in the live fixture; it does not alter behavior. The live test subsequently passed
with the final fixture. Cargo.lock, Cargo.toml, authentication, release workflows,
and production configuration were unchanged.

## Exercised contracts

The live test runs through real MCP handlers over an RMCP transport using isolated
synthetic authentication. It uploads and downloads a PNG, text attachment, and
synthetic video fixture; inserts image/file/video nodes and an HTTPS iframe embed;
reads rich JSON; lists exactly three referenced attachments; rejects Markdown
replacement and independently confirms the document is unchanged after rejection.

Original paragraph text and node ID survive append. Docmost drops a null
`textAlign` attribute on its first collaborative update. The fixture normalizes
only that verified no-op default, then compares the original node.

Mock tests cover filename/base64/size validation, multipart field ordering,
allowlisted metadata, same-page ownership, oversized download rejection, no
redirects, and no mutation retry on 401/403/500. Router and stdio compatibility
tests assert the exact thirteen-read/sixteen-write-supported inventories with
unchanged explicit write allowlisting.

## Limits and remaining delivery gates

- Upload/download limit: 2 MiB decoded bytes.
- Listing covers referenced files, not unattached uploads.
- No standalone file deletion, file replacement, or arbitrary JSON overwrite.
- Video bytes/node transfer is verified, not playback. External iframe rendering
  and a browser visual check were not performed.
- This is a connector-level live test, not a new Atlas confirmation-gate E2E.
- Independent review, provider CI/platform checks, and authorized release/install
  remain required before production rollout. Saved-login compatibility must be
  preserved when choosing the release lineage.

## Cleanup

All four task-owned containers (build, Docmost, PostgreSQL, Redis) were removed.
Their final three anonymous data volumes were confirmed absent, and the isolated
network was removed. Earlier failed-fixture stacks were also removed with their
anonymous volumes before each fresh run. Synthetic data is not retained; it can
be recreated from the opt-in fixture. Production containers were not restarted.
