# F-06 / IA-01 session identity remediation

Date: 2026-08-30

## Exact candidate

- Source commit: `e178c7d847987ecdca3c9bf076c0ec0cd481e83f`
- Source tree: `aab54d519e0bf3f32c730f5c7cf1ee0e9f272153`
- `Cargo.lock` blob: `abe8f39adadd7dde826fa8fdd046f4ee683d0a59`
- Base workflow integration commit: `7e772975ff8d8631a2059180498f14366ddb4b31`
- Reviewed Atlas MCP runtime commit: `efad3719b67fc9949be3809a7d07b297a64de10d`

## Source correction

`StoredSession` now carries an optional identity email. Automatic session reuse
requires an exact canonical-origin match and an exact session-email/config-email
match. Legacy sessions with no identity binding fail closed.

Login persistence now clears the prior same-origin session before writing the
new identity-bearing config and replacement session. Therefore a config write
or replacement-session write failure cannot leave an older token available for
restart. The post-login readback path applies the same origin and identity
binding before returning an authenticated session.

The state store has a test-only one-shot session-write failure injector. The
deterministic regression places identity A in the store, begins a session-only
transition to identity B, injects the failure at replacement-session
persistence, and verifies all of the following:

- identity B's config is present;
- identity A's old session is absent;
- remembered credentials are absent;
- the injected error is returned;
- a mismatched identity session is rejected;
- a legacy unbound session is rejected;
- only an exact origin and identity binding is accepted.

## Automated verification

The following commands passed against the exact candidate using the pinned
`rust:1.98.0-slim-bookworm` image at digest
`sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157`:

- `cargo fmt --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --no-default-features`
- `cargo test --locked --all-features`

No lockfile change occurred.

## Binary provenance

| Target | SHA-256 | Inspection |
| --- | --- | --- |
| Linux x86-64 | `4237827500f5fd51db2ce86b767bf4aeb4cdb803a5ed28ccb2723237c6e4a90e` | ELF 64-bit x86-64 PIE, stripped |
| Linux arm64 | `f27cf1d7626f1889d77710059e60ea7995e048ad8980955fc2eac0af805f4b46` | ELF 64-bit aarch64 PIE, stripped |

The arm64 artifact was built natively under Docker Desktop. The x86-64
artifact was built natively on the authorized Ubuntu host with the same pinned
Rust image after the Docker Desktop QEMU cross-build crashed the compiler. Both
artifacts were independently hashed after build.

## Fresh Docmost Community v0.95.0 evidence

The exact x86-64 artifact was tested against the isolated
`docmost-atlas-compat` stack, not the ordinary Docmost service.

- The credential and session lifecycle matrix used disposable homes and two
  restricted synthetic identities. Session-only identity B replaced remembered
  identity A, the remembered credentials were cleared, forced 401 required
  interactive B reauthentication, identity A was not silently reused, and a
  deliberately desynchronized stale identity failed closed before network
  login. Expiry, origin-scoped forget, loopback request validation, and security
  headers also passed. All lifecycle homes were removed.
- The default MCP inventory was exactly the ten read tools. All ten were
  exercised successfully and every mutation name was unavailable.
- The explicit write inventory exposed exactly the ten reads plus the five
  requested write tools. The synthetic page, move, and comment calls succeeded;
  every unallowlisted mutation remained unavailable. A fresh read-only process
  observed the updated page title/body, nested child, and one comment. As in the
  prior accepted evidence, `get_comments` does not render the comment body, so
  body-update observation is not claimed from that API response.

## Fresh Atlas control evidence

The exact reviewed Atlas runtime at `efad3719` registered the exact candidate
through the literal SSH stdio launch path.

- Ten of ten read calls were blocked before decision, approved once, consumed
  once, and dispatched successfully.
- The write registration exposed only `create_page` beyond the ten reads.
- One approved write dispatched exactly once; a separate denied write
  dispatched zero times.
- Confirmation projections contained no tested argument values.
- All temporary MCP registrations were removed.

The direct and Atlas-routed hostile matrices covered redirect, timeout,
declared oversize, chunked oversize, permission denial, and server error. Every
scenario failed closed, timeout stayed within the declared boundary, redirect
target hits remained zero, and no canary or private-origin value appeared in
the checked output.

## Cleanup and remaining boundary

The isolated Docmost containers and all three isolated volumes were removed;
readback returned zero containers and zero volumes for Compose project
`docmost-atlas-compat`. The ordinary Docmost stack on port 3001 was untouched.
The tailnet-only `:8443` Serve rule could not be disabled without interactive
sudo and now points to a closed loopback port; this is separately managed
infrastructure, contains no test data, and does not weaken the verified cleanup.

No branch was published, no pull request was created or merged, no release was
published, and no deployment was enabled by this remediation.
