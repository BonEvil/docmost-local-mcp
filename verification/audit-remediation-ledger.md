# Independent-audit remediation ledger

Date: 2026-08-30

Deployment state: **DISABLED**

This ledger maps the independent audit at `e96ae6e` to the exact post-remediation
candidate, the administrator-supplied provider read-back, and the refreshed live
evidence. It does not treat superseded evidence as proof for the current identity
and does not infer provider controls from repository YAML.

## Exact identities

| Identity | Exact value |
| --- | --- |
| Post-remediation source commit | `0e67438dbf975d1818b554ec10dfbd4905b84d84` |
| Post-remediation source tree | `f88b8117ffcf78295a00581c6a0c5264a37dada5` |
| `Cargo.lock` SHA-256 (unchanged by every remediation step) | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Debian snapshot policy SHA-256 | `805348a5d9a466214f8596eac0318be8e03b0269669f194a81c0834d3263e4c4` |
| Dockerfile SHA-256 | `8dc537b7581d311f74b5210a1de3d09e939326a07a9850bc280ac47ee05acc1e` |
| Linux x86-64 live-tested binary SHA-256 | `ff677008ca257de7feff1fefbddf5316d790149515a0decb618e80f05f0690d6` |
| Docmost server image digest | `docmost/docmost@sha256:41c8d777cf23c74e78f94e676aec328b7d7856f48df5e573543dac68d371e37c` |
| Atlas runtime commit / tree | `efad3719b67fc9949be3809a7d07b297a64de10d` / `58d8b8c5d330c905ef70b5be33b06883c0a57ae6` |

Superseded identities, retained only as history: source `f398ebb…` with arm64
binary `e0467973…`, and evidence commit `06bbe06…` with x64 binary `a7318362…`.
Both predate the credential-store fix below and are not evidence for this
candidate.

## Finding traceability

| Audit item | Change and test/evidence | Disposition | Residual risk |
| --- | --- | --- | --- |
| F-01 | Canonical-origin controls unchanged; both locked suites and the origin/auth regressions pass. Live runs used one canonical origin with origin-scoped session state at mode `0700`/`0600`. | Fixed | Same-origin identity risk is handled by IA-01. |
| F-02 | Installer negatives reject wrong manifest commit, wrong version, duplicate asset, malformed digest, digest mismatch, provenance failure, partial and oversize transfers, and unapproved redirects while preserving the existing executable. | Fixed | No fork release exists; publication remains unauthorized. |
| F-03 | Authority inventory and annotations unchanged. Live: exactly ten read-only tools by default, every one of the ten mutation names unavailable, an exact five-name write allowlist, and all five unallowlisted mutations unavailable. | Fixed | Docmost permissions remain the server-side blast-radius boundary. |
| F-04 | HTTPS-or-explicit-literal-loopback parsing and no-redirect clients unchanged. Live: a `302` was refused and the redirect target received zero requests. | Fixed | The loopback-HTTP override stays development-only. |
| F-05 | `cargo-deny 0.20.2` ran against the current advisory database and the unchanged lockfile: advisories, bans, licenses, and sources all `ok`. | Accepted with the existing exact `rmcp` exception | Advisory data moves; rescan at every candidate and release. |
| F-06 / IA-01 | Session-only login clears every remembered representation before committing, and reauthentication requires the remembered identity to match the active configuration. Live: identity B submitted session-only while both of A's remembered files were present removed them and switched the active identity; a forced 401 required interactive authentication and produced **zero** identity-A logins at the server; a desynchronized credential/config pair failed closed with the identity-mismatch error, cleared the stale credentials, and started no login flow. | Fixed and live-verified | Interactive re-enrollment is required after session-only expiry, by design. |
| F-06 follow-on | Refreshed evidence showed `remember_password=true` reporting success while retaining nothing, because the reviewed graph compiles no platform credential-store backend. `KeyringStore::write_credentials` now reads the secret back through an independent handle and fails closed when it is not retained, with a regression test and corrected operator documentation. Live: the request is refused with `Secure OS credential storage is unavailable.` and creates no state at all. | Newly fixed in this card | Unattended reauthentication now requires the acknowledged encrypted-file fallback; enabling a real platform backend would change the reviewed dependency graph and needs its own advisory review. |
| F-07 | Positive diagnostic allowlist unchanged. Live: hostile `403`/`500` bodies carrying credential, token, content, and address canaries produced only `Response body omitted (<n> bytes).`; no canary, origin, host, URL, or excerpt appeared in any message or in Atlas-visible output. | Fixed and live-verified | Free-form diagnostic arguments remain an upstream-sync review hazard. |
| F-08 / IA-03 | Live negative matrix on the exact binary: redirect refused, overall deadline fired at 30.2 s, declared-length and chunked oversize both rejected at the 8 MiB cap, permission denial and hostile server error reduced to safe classes. All six repeated through the Atlas confirmation path failed closed with zero leakage. | Fixed and live-verified | Tool failures surface as JSON-RPC errors, so Atlas ends the connection and reports a generic unavailability message instead of the specific safe message. |
| F-09 / IA-03 | Live loopback matrix: 32-byte flow secret on a literal IPv4 loopback address; wrong flow, `Host`, `Origin`, missing flow header, and non-JSON content type all rejected; `/success` displayed without settling; all required response headers present. Expiry, 401, and origin-scoped idempotent forget were exercised end to end. | Fixed and live-verified | None beyond the documented development-only loopback-HTTP override. |
| F-10 / IA-02 | Release accepts only a provider-verified annotated signed tag whose commit is reachable from the protected branch and carries exactly one successful terminal gate, with privileged publication bound to `protected-release`. Debian inputs are snapshot-pinned with exact package versions and fork metadata is corrected. Administrator read-back is recorded in `audit-remediation-provider-evidence.json`. | Fixed under the authorized single-maintainer model | The zero-human-approval model depends on the exact terminal gate, enforced admins, immutable tag and release controls, and protected-release approval staying configured. A real release remains unauthorized and untested. |

## Repository checks on the exact candidate

All of the following passed against `0e67438` in the pinned Rust 1.98.0 container
or on the host:

- `cargo fmt --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-features` — 136 tests, zero failures
- `cargo test --locked --no-default-features` — 136 tests, zero failures
- lib-test suite repeated six times for ordering stability
- `cargo deny check advisories bans licenses sources` (cargo-deny 0.20.2) — all `ok`
- `bash tests/release_integrity_test.sh`
- `bash tests/release_context_test.sh`
- `bash scripts/check-pinned-inputs.sh`
- CI and release workflow YAML parsed with the expected job sets
- `git diff --check` and a clean worktree

## Hosted gate on the exact candidate

Draft pull request #2 published source candidate `0e67438` and evidence commit
`9e01947` on the card branch. All 13 hosted check runs on `9e01947` completed
successfully in Actions run `33318452752`: six platform builds, three Rust
check configurations, dependency policy, launcher smoke, release integrity, and
the terminal `Integrated security and release gate`. The pull request remains
draft, unmerged, and targeted at the protected branch.

## Refreshed live evidence

The complete IA-03 matrix was re-executed on binary
`ff677008ca257de7feff1fefbddf5316d790149515a0decb618e80f05f0690d6`, built from
`0e67438` inside the digest-pinned Rust image and digest-verified before use and
after cleanup. Results and sanitized machine-checkable facts are in
[`ia03-refreshed-live-evidence.md`](ia03-refreshed-live-evidence.md) and
[`ia03-refreshed-live-evidence.json`](ia03-refreshed-live-evidence.json):

- Docmost Community v0.95.0 compatibility with all ten read tools, an exact
  five-name write allowlist, and independent database confirmation.
- Atlas-controlled operation: ten gated reads approved and dispatched once each,
  no write name reachable by default, and an approve-once/deny pair producing
  exactly one dispatch and one page with no duplicate.
- The full runbook negative and lifecycle matrix, including the IA-01 rollback
  sequence measured at the server.
- Cleanup: zero temporary Atlas registrations, zero registrations in the live
  Atlas instance, all disposable runtimes and harness artifacts removed, the
  isolated stack unchanged at three containers and three volumes, and the
  separate production project unchanged at three containers and never accessed.

## Provider-control resolution

The audit-time provider snapshot is preserved separately and not rewritten. The
administrator read-back for the authorized single-maintainer model is recorded in
`audit-remediation-provider-evidence.json`: protected `main` requiring pull
requests plus the strict exact `Integrated security and release gate` with admins
enforced; stale-review dismissal, conversation resolution, and linear history
enabled; force-push and deletion disabled; an active ruleset protecting
`refs/tags/v*` from update and deletion with no bypass; immutable releases
enabled; a `protected-release` environment requiring approval and restricted to
protected branches; Actions limited to the nine reviewed full-SHA references; and
a corrected fork homepage.

## Deployment boundary

No merge, tag, release, production credential or content access, production Atlas
registration, installation, or deployment was performed. The draft pull request
remains draft and unmerged. Deployment stays disabled pending the final
independent gate, which must reassess F-01 through F-10 and IA-01 through IA-03
against this exact candidate. Any source, lockfile, or build-input change creates
a new identity and requires a complete rebuild and rerun.
