# Final independent post-remediation audit

Audit date: 2026-08-30

Repository: `BonEvil/docmost-local-mcp`

Decision: **NOT READY — one deployment-blocking IA-01 regression remains**

Deployment state: **DISABLED**

## Executive decision

The post-remediation candidate closes F-01 through F-05 and F-07 through F-10
with source, automated, provider, Docmost Community v0.95.0, and Atlas evidence
that is coherent with the exact reviewed identities. The new keyring read-back
guard also fails closed when the compiled credential store reports success but
retains no secret.

The candidate does **not** close F-06 / IA-01 completely. A session-only login
clears remembered credentials, then writes the new identity's configuration,
then writes the new session. If the final session write fails, the old
same-origin session remains alongside the new identity's configuration. On a
restart, `get_authenticated_session` checks the session's origin and expiry but
does not bind the session token to the configured identity; it therefore returns
the old token while labeling the authenticated session with the new identity.
This is precisely the multi-file failure window that IA-01 required the
remediation to close. It can silently restore the previous account's authority
and is deployment-blocking.

No code was changed by this audit. The only additions are this report and the
concise readiness report.

## 1. Exact review boundary and independently checked identities

| Item | Exact value | Review result |
| --- | --- | --- |
| Assigned review commit | `7e772975ff8d8631a2059180498f14366ddb4b31` | Inspected |
| Assigned review tree | `ba7a18fcd77d272ef691fa032c749d14505cf6b1` | Recomputed |
| Evidence head | `da411536e744d69cc910562ff1ee01cd40ecfb04` | Same assigned tree |
| Evidence head tree | `ba7a18fcd77d272ef691fa032c749d14505cf6b1` | Recomputed |
| Live-tested source commit | `0e67438dbf975d1818b554ec10dfbd4905b84d84` | Inspected |
| Live-tested source tree | `f88b8117ffcf78295a00581c6a0c5264a37dada5` | Recomputed |
| Source-to-review drift | Three `verification/` files only | No source, test, workflow, configuration, dependency, lockfile, or build-input drift |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` | Recomputed and matched |
| Dockerfile SHA-256 | `8dc537b7581d311f74b5210a1de3d09e939326a07a9850bc280ac47ee05acc1e` | Recomputed and matched |
| Debian snapshot policy SHA-256 | `805348a5d9a466214f8596eac0318be8e03b0269669f194a81c0834d3263e4c4` | Recomputed and matched |
| Live-tested Linux x86-64 binary SHA-256 | `ff677008ca257de7feff1fefbddf5316d790149515a0decb618e80f05f0690d6` | Bound by retained pre-use and post-cleanup evidence to source `0e67438`; binary is not retained locally |
| Build command | `cargo build --locked --release --no-default-features` | Recorded in refreshed evidence |
| Build image | `rust:1.98.0-slim-bookworm@sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157` | Digest-pinned in source and refreshed evidence |
| Docmost target | Community v0.95.0, image digest `sha256:41c8d777cf23c74e78f94e676aec328b7d7856f48df5e573543dac68d371e37c` | Exact retained live target |
| Atlas runtime | commit `efad3719b67fc9949be3809a7d07b297a64de10d`, tree `58d8b8c5d330c905ef70b5be33b06883c0a57ae6` | Recomputed in retained live run |

`git diff --name-status 0e67438..7e77297` contains only
`verification/audit-remediation-ledger.md`,
`verification/ia03-refreshed-live-evidence.md`, and
`verification/ia03-refreshed-live-evidence.json`. Older identities
`f398ebb…`, `e0467973…`, and `a7318362…` are correctly marked superseded and
were not used as proof for the current binary.

## 2. Checks and evidence inspected

Fresh checks in this card runtime:

- `git diff --check`: passed.
- `bash tests/release_integrity_test.sh`: passed.
- `bash tests/release_context_test.sh`: passed.
- `bash scripts/check-pinned-inputs.sh`: passed.
- Git object connectivity (`git fsck --no-dangling --no-progress`): passed.
- Remote ref inspection: `main` remains `0bb296068227…`; pull request ref
  `refs/pull/2/head` is `da411536e744…`; all ten existing `v*` tags resolve to
  ancestor commits and none resolves to this candidate.

Cargo is unavailable in this card runtime, so this audit does not misstate the
retained Cargo results as fresh local execution. The exact-candidate retained
evidence reports:

- format and warnings-denied Clippy passed;
- both locked feature-mode suites passed with 136 tests and zero failures;
- the library suite passed six repeated runs;
- cargo-deny 0.20.2 reported advisories, bans, licenses, and sources all `ok`;
- all 13 hosted checks succeeded on evidence commit `9e01947` and again on
  `da41153`, including six platform builds and the terminal integrated gate.

The Atlas provider connector could not perform a new authenticated read because
two configured identities can access this repository and no binding is selected.
This audit did not guess or mutate that binding. Provider-control conclusions
therefore rely on the committed administrator read-back produced by the
remediation card; the public Git ref checks above independently confirm the
main, pull-request-head, and tag boundaries that Git exposes.

## 3. Finding closure

| Finding | Final disposition | Direct basis | Residual or blocker |
| --- | --- | --- | --- |
| F-01 — cross-origin credentials | Fixed | Canonical origin parser, origin-scoped config/session/credential paths, process pinning, no-redirect clients, tests and live origin-scoped state | No blocker found |
| F-02 — unverified release executable | Fixed for the release design | Commit/version/digest-bound manifest, Sigstore verification, bounded atomic installer, negative tests, exact source-to-binary live provenance | No real fork release has been authorized or tested; release remains gated |
| F-03 — ambient write authority | Fixed | Exact ten-read default inventory, explicit mode plus nonempty exact allowlist, conservative annotations, all mutations unavailable by default, Atlas approve/deny dispatch ledger | Docmost permissions remain the server-side blast-radius boundary |
| F-04 — plaintext credential transport | Fixed | HTTPS default, literal-loopback-only explicit HTTP exception, redirects disabled; live redirect target received zero requests | Development-only loopback override must remain explicit |
| F-05 — dependency/advisory policy | Accepted exception, non-blocking | Exact lockfile, current retained cargo-deny pass, no native webview graph, `rmcp` limited to stdio path | `RUSTSEC-2026-0189` acceptance expires if rmcp version/features/transport or advisory scope changes; rescan every candidate/release |
| F-06 / IA-01 — credential and identity lifecycle | **Blocking** | Successful paths clear credentials and reject mismatched remembered identities, but the config/session commit is not atomic and the saved session has no identity binding | **Old same-origin token can survive a failed new-session write and be accepted under the new identity after restart** |
| F-07 — private-data diagnostics | Fixed | Positive diagnostic allowlist; live hostile bodies reduced to safe status/length classes with no canary, URL, origin, host, or excerpt | Free-form static message/scope parameters require upstream-sync review |
| F-08 — deadlines and limits | Fixed | 5 s connect/30 s overall deadlines, 8 MiB caps, bounded inputs/results, live direct and Atlas negative matrix | JSON-RPC failure shape degrades Atlas diagnosis and marks the server unavailable, but remains fail-closed and leak-free |
| F-09 — loopback authentication | Fixed | 32-byte secret, exact Host/Origin/header/content-type checks, display-only success, restrictive headers, expiry/401/forget live matrix | Development-only loopback HTTP exception remains documented |
| F-10 / IA-02 — provenance/governance | Fixed under recorded single-maintainer model | Signed annotated tag check, protected-branch ancestry, exact terminal gate, protected release environment, snapshot/version-pinned Debian inputs, full-SHA Actions, corrected fork metadata, administrator provider read-back | Zero-human-review model depends on those provider controls remaining configured; a real release is still unauthorized |

### Deployment-blocking F-06 / IA-01 path

The failure sequence is source-demonstrable:

1. Origin O already has identity A's unexpired saved session.
2. Identity B successfully authenticates with `remember_password=false`.
3. `login` clears remembered credentials at `src/auth/manager.rs:228-233`.
4. It writes B's `StoredConfig` at `src/auth/manager.rs:234-240`.
5. It then writes B's `StoredSession` at `src/auth/manager.rs:241-248`.
6. If step 5 fails—for example, writing, permission-setting, or renaming the
   session file fails—the function returns an error but A's prior session file
   is not removed or rolled back.
7. After restart, `get_authenticated_session` reads B's config and A's session.
   At `src/auth/manager.rs:87-99` it verifies only canonical origin and expiry,
   then `to_authenticated_session` combines B's configured email with A's token.

`StoredSession` has an origin but no identity field, and no failure-injection
test covers a failure between `write_config` and `write_session`. The successful
live A-to-B sequence cannot prove this failure path safe. The source comment
claiming that a later persistence failure cannot restore the old identity is
therefore false for the saved-session path.

Closure requires a new source candidate that makes config/session replacement
atomic or removes/invalidates the old session before a new identity can become
active, binds reusable sessions to the active identity (or independently
verifies it), and adds a deterministic failure-injection regression covering
the write boundary. Because this card is an independent review, implementing
that fix is out of scope.

## 4. Compatibility, Atlas, cleanup, and production boundary

The refreshed evidence is correctly bound to binary `ff677008…` and reports:

- Docmost Community v0.95.0: all ten reads exercised; every mutation unavailable
  by default; exact five-name write allowlist; all unallowlisted mutations
  unavailable; two pages, one nested child, and one updated comment independently
  confirmed in the disposable database.
- Atlas: all ten gated reads held before decision and each dispatched once after
  approval; no write name reachable by default; confirmation projections
  contained no argument values; one approved `create_page` produced one dispatch
  and one page; denial produced zero dispatches and no duplicate.
- Negative/lifecycle matrix: redirect refused with zero target requests; timeout
  at 30.2 s; declared and chunked oversize rejected at 8 MiB; 403 and 500 bodies
  reduced to safe classes; loopback, expiry, 401, remembered-password fail-closed,
  successful identity rollover, stale-credential guard, and scoped forget passed.
- Cleanup: zero temporary or live Atlas registrations and pending authorizations;
  disposable Atlas databases/source export/HOMEs/harness artifacts removed;
  isolated stack remained three containers and three volumes; reviewed binary
  digest unchanged.
- Ordinary production: three containers before and after; no production endpoint,
  identity, database, page, space, credential, configuration, or content accessed
  or changed by the retained runs. This audit likewise used no production access.

These verified successful paths remain valid for the exact current binary, but
they do not negate the source-demonstrable persistence-failure window.

## 5. Regression-risk review

| Area | Assessment |
| --- | --- |
| Identity transition | **High / deployment-blocking:** split config/session persistence can pair the new identity with the old same-origin token after a failed session write |
| Credential-store behavior | Accepted with explicit limitation: the supported headless build has no durable platform backend and fails closed; acknowledged encrypted-file fallback is needed for unattended reauthentication |
| Authority and Atlas dispatch | Low for this exact inventory and runtime; any new write must update router, annotations, allowlist tests, and Atlas confirmation evidence |
| Network and diagnostics | Low in current source; JSON-RPC error termination remains an operator-diagnosis degradation |
| Dependencies | Medium accepted: advisory data moves and the exact unreachable rmcp exception must be refreshed |
| Release governance | Medium accepted: safety depends on provider settings outside the tree remaining enforced; first release still needs separate approval and evidence |
| Compatibility | Low for binary `ff677008…`; any source, lockfile, or build-input change invalidates all live evidence |
| Disposable content | Accepted: synthetic pages/comments remain only in the disposable instance because the reviewed surface has no delete operation |
| Upstream sync | Medium procedural risk: every sync must record the range and repeat invariant, automated, compatibility, Atlas, and independent gates |

## 6. Readiness and delivery boundary

- Deployment remains disabled.
- Merge, tag creation, release, installation, production credential/content access,
  Atlas production registration, and deployment remain unauthorized.
- This audit does not clear repository delivery while the F-06 / IA-01 blocker
  remains. After a new candidate is fixed and fully re-evidenced, the maximum
  authorized repository delivery remains an **unmerged pull request targeting
  `BonEvil/main`**. Merge and deployment require separate approval.
- Any source, lockfile, dependency, workflow, or build-input change creates a new
  candidate and requires new digests, automated checks, Docmost v0.95.0 evidence,
  Atlas evidence, cleanup evidence, and a fresh independent final audit.
