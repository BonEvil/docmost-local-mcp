# IA-03 refreshed live evidence

Refresh date: 2026-08-30

Deployment state: **DISABLED**

This report records the mandatory IA-03 live re-verification on the exact
post-remediation candidate. Machine-checkable facts are in
[`ia03-refreshed-live-evidence.json`](ia03-refreshed-live-evidence.json). Both
files are sanitized: they contain no endpoint, host, port, credential, token,
cookie, session filename, page or space identifier, private body, or synthetic
canary value.

## Exact tested identities

| Identity | Value |
| --- | --- |
| Source commit | `0e67438dbf975d1818b554ec10dfbd4905b84d84` |
| Source tree | `f88b8117ffcf78295a00581c6a0c5264a37dada5` |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Linux x86-64 binary SHA-256 | `ff677008ca257de7feff1fefbddf5316d790149515a0decb618e80f05f0690d6` |
| Build command | `cargo build --locked --release --no-default-features` |
| Build image | `rust:1.98.0-slim-bookworm@sha256:1469a27c…f157` |
| Docmost server | Community `0.95.0`, image digest `sha256:41c8d777…e37c` |
| Atlas runtime commit / tree | `efad3719b67fc9949be3809a7d07b297a64de10d` / `58d8b8c5d330c905ef70b5be33b06883c0a57ae6` |
| Negotiated MCP protocol | `2025-03-26` |

The binary was built from the exact commit inside the digest-pinned Rust image,
its digest was recorded before testing, and the same digest was recalculated
after every run and after cleanup. The Atlas runtime was exported read-only from
Git and its recomputed tree hash matched the reviewed value exactly.

## 1. Docmost Community v0.95.0 compatibility

A default process negotiated `2025-03-26` and exposed exactly the ten read tools,
every one annotated read-only. All ten mutation names — the five allowlistable
ones plus `create_space`, `update_space`, `duplicate_page`, `copy_page_to_space`,
and `move_page_to_space` — returned `tool not found` in default mode.

All ten read tools were exercised against the isolated instance. A separately
launched process with `--authority-mode=write` and the five-name allowlist
exposed exactly fifteen tools, none of the five unallowlisted mutations, and
marked every mutation as not read-only. It created a synthetic parent and child
page, updated the child, nested it under the parent, and created and updated one
comment. A fresh default read-only process then confirmed the updated title and
body, one nested child, and one comment.

A direct read-only database query confirmed two synthetic pages, the updated
child title, one nested child, one synthetic comment, and the updated comment
body. `get_comments` never renders comment bodies, so the comment update was
confirmed by that independent query rather than by tool output.

## 2. Atlas-controlled end-to-end operation

Atlas launched the absolute reviewed binary through literal argument vectors with
no shell, npm, downloader, or credential argument.

- Default registration: exactly the ten read tools, all read-only, and no write
  name present in either the MCP inventory or the Atlas tool registry.
- Each of the ten read tools was dispatched through the real Atlas generic-MCP
  path. Every call entered the mandatory confirmation, remained undispatched
  before the decision, was approved once, and recorded `dispatch_succeeded`.
  Ten approvals produced ten dispatches.
- No confirmation projection contained any supplied argument value.
- A separate write registration with the smallest allowlist exposed the ten reads
  plus exactly `create_page`. The approved call dispatched once; a second
  identical call was denied, never dispatched, returned
  `mcp_confirmation_denied`, and retained a null dispatch outcome.
- An independent database query found exactly one Atlas-gated page, created by
  the disposable identity in the disposable space, and zero duplicates from the
  denied call.
- Zero MCP registrations remained after the run.

## 3. Runbook negative and lifecycle matrix

### Network negative paths

Each path used a dedicated hostile loopback origin whose bodies embedded
synthetic credential, token, content, and address canaries. Every scenario failed
closed and leaked nothing:

| Scenario | Observed result | Elapsed |
| --- | --- | --- |
| Redirect | `Docmost API request failed (302 Found).` and the redirect target received zero requests | 0.2 s |
| Overall timeout | `Failed to call /api/spaces` at the declared 30-second deadline | 30.2 s |
| Declared oversize | `Docmost success response body exceeded the 8388608-byte limit.` | 0.6 s |
| Chunked oversize | same bounded-cap rejection with no declared length | 0.2 s |
| Permission denied | `Docmost API request failed (403 Forbidden). Response body omitted (718 bytes).` | 0.2 s |
| Hostile server error | `Docmost API request failed (500 Internal Server Error). Response body omitted (718 bytes).` | 0.2 s |

No message contained a canary, an origin, a host, a URL, or a response excerpt.
The process stayed alive after every failure. The same six scenarios were then
repeated through the Atlas confirmation path; all six failed closed with no
canary, origin, or URL in any Atlas-visible output.

### Credential and loopback lifecycle

- **Loopback flow.** A fresh 32-byte flow secret on a literal IPv4 loopback
  address. Wrong flow, wrong `Host`, wrong `Origin`, missing flow header, and a
  non-JSON content type were all rejected (`403`, and `415` for the content-type
  case) without invoking the login handler. `/success` with a valid flow rendered
  but did not settle the login. Responses carried `no-store`, `no-referrer`,
  `nosniff`, `DENY`, and a CSP with `frame-ancestors`, `form-action`, and
  `base-uri` set to `'none'`. The valid submission produced session-only state
  with directory mode `0700`, files `0600`, and no credential file.
- **Session expiry and 401.** Both forced conditions required interactive
  authentication instead of silent credential reuse, and both recovered cleanly.
- **Remembered-password persistence.** With no acknowledged fallback,
  `remember_password=true` fails closed with
  `Secure OS credential storage is unavailable.` and creates no state directory,
  session, or credential file. A session-only retry then succeeds.
- **Same-origin identity rollback (IA-01).** Identity A was remembered under the
  acknowledged fallback, producing an origin-scoped ciphertext and key at mode
  `0600`. Identity B was then submitted session-only while both of A's remembered
  files were present. After B's login, both files were gone and the active
  configuration was B. A forced 401 required interactive authentication, left no
  credential file, and identity A received **zero** reauthentications measured at
  the server. A desynchronized state — A's credentials with B's active
  configuration — failed closed with
  `do not match the active Docmost identity`, cleared the stale credentials,
  started no loopback prompt, and again produced zero identity-A logins.
- **Origin-scoped forget.** Two origins held two sessions and four credential
  files. Forgetting the first origin removed only its session and credential
  files and left the second origin intact; a second identical run also exited
  zero. Forgetting the second origin removed all remaining state.

## 4. Containment and cleanup

Pre- and post-run assertions: the isolated Compose project held exactly three
containers and three volumes throughout; the separate production project held
exactly three containers before and after and was never targeted, read, or
changed. No production endpoint, identity, database, page, space, credential, or
configuration was accessed.

After the runs: zero temporary Atlas MCP registrations, zero registrations of
this server in the live Atlas instance, zero pending authorizations, all isolated
Atlas runtime databases and the read-only source export deleted, every disposable
HOME and harness artifact removed from the isolated host, and the installed
binary digest unchanged. The ephemeral test password remained only in its
host-side mode-`0600` file and was never printed, copied, or written into any
artifact. The shared host state directory remained mode `0700` with zero
credential files, confirming session-only operation.

## 5. Residual risk

- Synthetic pages and comments remain inside the disposable instance because the
  reviewed tool surface exposes no delete operation. They are contained by the
  disposable stack, which is the containment boundary.
- The reviewed build compiles no platform credential-store backend, so remembered
  passwords always fail closed on the supported host. Unattended reauthentication
  therefore requires the explicitly acknowledged encrypted-file fallback. Enabling
  a real platform backend would change the reviewed dependency graph and needs its
  own advisory review.
- The server reports tool failures as JSON-RPC errors rather than tool results
  with `isError`. Atlas therefore ends the connection and reports a generic
  "server stopped responding" instead of the specific safe message. This is
  fail-closed and leaks nothing, but it degrades operator diagnosis and marks the
  server unhealthy after any ordinary Docmost failure.
- Live evidence binds to binary `ff677008…`. Any source, lockfile, or build-input
  change invalidates it and requires a full rebuild and rerun.
