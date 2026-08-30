# Final independent post-remediation audit

Audit date: 2026-08-30

Repository: `BonEvil/docmost-local-mcp`

Decision: **READY FOR PR-ONLY REPOSITORY DELIVERY**

Deployment state: **DISABLED**

## Executive decision

The exact resumed candidate closes the final F-06 / IA-01 persistence-failure
window without reopening F-01 through F-05 or F-07 through F-10. Persisted
sessions are now bound to canonical origin and authenticated identity; legacy
unbound and mismatched sessions fail closed; the prior same-origin session is
invalidated before either the new config or replacement session is written; and
a deterministic injected replacement failure proves that identity A's token is
not recoverable after an attempted transition to identity B.

Fresh candidate-bound automated, binary-provenance, Docmost Community v0.95.0,
Atlas-control, hostile-path, lifecycle, cleanup, and ordinary-production
non-modification evidence is present. No unresolved deployment-blocking finding
remains across F-01 through F-10 or the regression-risk review.

This decision authorizes only the workflow's separately bounded repository
delivery to an **unmerged pull request targeting `BonEvil/main`**. It does not
authorize merge, tag creation, release, installation, production access,
production Atlas registration, or deployment. Deployment remains disabled
pending separate approval.

## 1. Exact review and provenance boundary

| Item | Exact value | Independent result |
| --- | --- | --- |
| Assigned final-card HEAD | `459c27f7a610d557995b2f4f02c271c2338760ec` | Inspected |
| Assigned HEAD tree before this report update | `6d7f75add45051e43f7bb82dee86818ba5ea3138` | Recomputed |
| Workflow integration | `5aa008fabc892131d91ef0478ebe928f0c5515d2` | Production source identical to assigned HEAD |
| Workflow integration tree | `5320aecbe29a3b8fc5952e879cdcb63e59ac67c5` | Recomputed |
| Integrated correction | `cbfea89` | Source correction inspected |
| Live-reviewed remediation commit | `e178c7d847987ecdca3c9bf076c0ec0cd481e83f` | Source-equivalent to `cbfea89` |
| Corrected source tree | `aab54d519e0bf3f32c730f5c7cf1ee0e9f272153` | Recomputed for both `cbfea89` and `e178c7d`; byte-for-byte diff is empty |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` | Recomputed; unchanged |
| `Cargo.lock` Git blob | `abe8f39adadd7dde826fa8fdd046f4ee683d0a59` | Matches refreshed evidence |
| Dockerfile SHA-256 | `8dc537b7581d311f74b5210a1de3d09e939326a07a9850bc280ac47ee05acc1e` | Recomputed; unchanged |
| Debian snapshot policy SHA-256 | `805348a5d9a466214f8596eac0318be8e03b0269669f194a81c0834d3263e4c4` | Recomputed; unchanged |
| Linux x86-64 binary SHA-256 | `4237827500f5fd51db2ce86b767bf4aeb4cdb803a5ed28ccb2723237c6e4a90e` | Fresh live-tested artifact |
| Linux arm64 binary SHA-256 | `f27cf1d7626f1889d77710059e60ea7995e048ad8980955fc2eac0af805f4b46` | Fresh native artifact |
| Build command | `cargo build --locked --release --no-default-features` | Recorded for exact corrected source |
| Build image | `rust:1.98.0-slim-bookworm@sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157` | Digest-pinned |
| Docmost target | Community v0.95.0 | Isolated compatibility target |
| Atlas runtime | `efad3719b67fc9949be3809a7d07b297a64de10d` | Exact reviewed runtime |

`git diff --exit-code e178c7d..cbfea89` is empty. From `cbfea89` to the
assigned HEAD, changes are confined to four `verification/` files: the two new
F-06 evidence files and this card's two final reports. There is no source, test,
workflow, dependency, lockfile, configuration, or build-input drift after the
live-reviewed source tree.

The earlier x86-64 binary `ff677008…` and its source `0e67438…` remain valid
historical evidence for the preceding candidate but are superseded for final
readiness by source tree `aab54d5…` and binary `42378275…`.

## 2. Fresh inspection and automated evidence

Fresh checks in this final-card runtime:

- source equivalence between `e178c7d` and `cbfea89`: passed;
- source/build-input drift inspection from `cbfea89` through assigned HEAD:
  verification files only;
- `git diff --check`: passed;
- `bash tests/release_integrity_test.sh`: passed;
- `bash tests/release_context_test.sh`: passed;
- `bash scripts/check-pinned-inputs.sh`: passed;
- Git object connectivity: passed;
- working tree was clean before the final report revision.

Cargo is unavailable in this independent runtime, so retained Cargo results are
not mislabeled as fresh local execution. The exact corrected-source evidence in
`verification/f06-session-identity-remediation.md` records successful execution
in the digest-pinned Rust 1.98.0 image of:

- `cargo fmt --check`;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`;
- `cargo test --locked --all-features`;
- `cargo test --locked --no-default-features`.

The lockfile did not change. Existing release-integrity, dependency-policy,
platform, and provider-control evidence remains applicable because the F-06
correction changed only authentication source, tests, and lifecycle docs; the
fresh locked tests and both native Linux artifacts bind the corrected source.

## 3. F-01 through F-10 final disposition

| Finding | Final disposition | Evidence and rationale | Residual risk |
| --- | --- | --- | --- |
| F-01 — credentials crossing origins | Fixed | Canonical origin parser, origin-scoped files/keyring identity, process pinning, no-redirect clients, tests and live origin-scoped state | None blocking |
| F-02 — unverified release executable | Fixed for the authorized release design | Commit/version/digest-bound manifest, Sigstore verification, bounded atomic installer, negative tests, exact source-to-binary provenance | No real fork release has been authorized or exercised |
| F-03 — ambient write authority | Fixed | Exact ten-read default inventory; explicit mode plus exact nonempty allowlist; all mutations unavailable by default; Atlas approve/deny ledger | Docmost permissions remain the server-side blast-radius boundary |
| F-04 — plaintext credential transport | Fixed | HTTPS by default, literal-loopback-only explicit exception, redirects disabled, zero redirect-target hits in hostile runs | Loopback HTTP remains development-only |
| F-05 — dependency/advisory baseline | Accepted narrow exception, non-blocking | Exact unchanged lockfile, cargo-deny evidence, no native webview graph, affected `rmcp` Streamable HTTP path unreachable from supported stdio builds | Rescan every candidate/release and revisit on rmcp feature/version/transport change |
| F-06 / IA-01 — credential and session identity | **Fixed and regression-verified** | Session identity field, exact origin/email matching, legacy/mismatch refusal, pre-transition old-session invalidation, injected write-failure regression, fresh identity-rollover/401/expiry/forget evidence | Interactive re-enrollment after session-only expiry is intentional; no blocker |
| F-07 — private diagnostics | Fixed | Positive diagnostic allowlist; hostile bodies reduced to safe classes with no canary, URL, origin, host, or excerpt | Free-form diagnostic messages remain an upstream-sync review hazard |
| F-08 — deadlines and bounds | Fixed | Connect/overall deadlines, body/input/output caps, direct and Atlas hostile matrices | JSON-RPC error termination degrades operator diagnosis but stays fail-closed and leak-free |
| F-09 — loopback authentication | Fixed | 32-byte secret, exact Host/Origin/header/content-type validation, display-only success, security headers, live lifecycle | Explicit literal-loopback override remains development-only |
| F-10 / IA-02 — provenance and governance | Fixed under recorded single-maintainer model | Signed annotated tag check, protected ancestry, exact terminal gate, protected release environment, snapshot/version-pinned Debian inputs, full-SHA Actions, corrected metadata, administrator read-back | Provider controls must remain enforced; first release still requires separate authorization |

### F-06 / IA-01 closure inspection

The previously reported failure window is closed in source:

1. `StoredSession` now carries optional `email`; optionality preserves parsing of
   legacy files only so they can be rejected safely.
2. `saved_session_matches_config` requires the config's canonical origin, the
   session origin, and the session/config email to match exactly. A legacy
   session with no email and a session for another identity both fail.
3. `persist_authenticated_state` clears remembered credentials for session-only
   login, then clears the prior same-origin session **before** writing either the
   new config or the new identity-bearing session.
4. If config or replacement-session persistence fails, no old token remains for
   restart. The post-interactive-login readback applies the same identity check.
5. The one-shot test failure injector forces the replacement-session write to
   fail after identity B's config is written. The regression asserts identity
   A's old session is absent, credentials are absent, the error is returned, and
   only exact origin-and-email binding is reusable.

The original concern was not merely hidden by successful-path evidence; it is
addressed by source ordering, explicit state binding, and deterministic failure
coverage.

## 4. Docmost v0.95.0, Atlas behavior, and hostile matrix

The refreshed F-06 evidence binds to Linux x86-64 binary `42378275…` and records:

- **Docmost Community v0.95.0:** all ten reads exercised successfully; default
  inventory exactly ten reads; every mutation unavailable by default; the write
  process exactly ten reads plus five selected mutations; all unallowlisted
  mutations unavailable; fresh read-only inspection confirmed updated page,
  nested child, and comment state.
- **Identity lifecycle:** session-only identity B replaced remembered identity A;
  credentials were cleared; forced 401 required interactive B authentication;
  identity A was never silently reused; stale/desynchronized identity failed
  closed before network login; expiry, scoped forget, loopback validation, and
  headers passed; disposable lifecycle homes were removed.
- **Atlas control:** ten reads were each held before decision, approved once,
  consumed once, and dispatched successfully; write inventory added only
  `create_page`; one approved write dispatched once; a denied write dispatched
  zero times; no tested argument appeared in confirmation; registrations ended
  at zero.
- **Hostile direct and Atlas paths:** redirect, timeout, declared oversize,
  chunked oversize, permission denial, and server error all failed closed;
  redirect target hits were zero; timeout stayed within policy; no canary or
  private-origin value leaked.

## 5. Cleanup and ordinary production boundary

Verified facts from the refreshed evidence:

- isolated Compose project `docmost-atlas-compat` ended with zero containers and
  zero volumes;
- disposable homes and temporary Atlas registrations were removed;
- ordinary Docmost on port 3001 was not targeted, read, or modified;
- no ordinary page, space, identity, database, credential, configuration, or
  content change occurred;
- no branch publication, pull-request mutation, merge, tag, release,
  installation, production Atlas registration, or deployment occurred.

Separately, a tailnet-only `:8443` proxy rule could not be disabled without
interactive sudo. It points to a closed loopback port, contains no data, and is
outside the repository/runtime acceptance boundary. This is a residual
infrastructure-cleanup item, not evidence of a running candidate, retained test
content, production modification, or deployment blocker.

## 6. Regression-risk and exception separation

### Verified facts

- No unresolved F-01 through F-10 finding remains for the exact corrected tree.
- Session reuse now requires exact origin and identity binding.
- Exact-candidate automated, Docmost, Atlas, negative-path, lifecycle, and cleanup
  evidence exists for the new binary digest.
- Deployment remains disabled and current remote `main` remains at baseline
  `0bb296068227…`; no candidate tag exists.

### Accepted residual risks and exceptions

- The headless build has no durable platform credential-store backend;
  remember-password fails closed unless the acknowledged encrypted-file fallback
  is enabled. Adding a backend changes the dependency graph and requires review.
- `RUSTSEC-2026-0189` is accepted only for the exact unreachable rmcp HTTP-server
  path; advisory data and reachability must be refreshed every candidate/release.
- Atlas reports generic unavailability after JSON-RPC tool errors, reducing
  diagnosis without weakening fail-closed behavior or exposing private data.
- Synthetic pages/comments remain only in the disposable instance because the
  reviewed MCP surface has no delete operation; the disposable stack is the
  containment boundary.
- Release safety depends on the recorded provider settings remaining enforced;
  no release evidence can exist until a separately authorized first release.
- The inert tailnet proxy rule should be removed by infrastructure operations
  when interactive sudo is available, but it contains no data and targets no
  listening process.

### Remaining blockers

None for PR-only repository delivery of this exact candidate.

## 7. Final readiness and delivery boundary

- The candidate is cleared for repository delivery only to an **unmerged pull
  request targeting `BonEvil/main`**.
- The currently visible pull-request ref still points to the older evidence head;
  this audit did not publish or mutate it. The workflow's terminal delivery step
  must preserve the corrected source and final evidence when it updates or opens
  the unmerged pull request.
- Merge, tag creation, release publication, installation, production credentials
  or content access, production Atlas registration, and deployment remain
  separately unauthorized.
- Deployment remains disabled pending separate approval.
- Any source, lockfile, dependency, workflow, or build-input change creates a new
  candidate and invalidates this readiness decision until the complete automated,
  provenance, Docmost, Atlas, cleanup, and independent gates are rerun.
