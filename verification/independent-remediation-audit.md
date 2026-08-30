# Independent post-implementation security audit

Audit date: 2026-08-30

Repository: `BonEvil/docmost-local-mcp`

Review type: independent source, test, CI, release, compatibility, Atlas, and provider-evidence review

Deployment verdict: **BLOCKED**

## Executive verdict

The hardened fork substantially closes the original audit. F-01, F-02, F-03,
F-04, F-07, F-08, and F-09 are fixed in the reviewed source and supported by
candidate-bound automated or live evidence. F-05 is accepted with one narrow,
source-supported `rmcp` transport exception and a mandatory fresh advisory scan
at the next candidate. Two control families remain blocking:

1. **F-06 / IA-01 — remembered-identity rollback.** A session-only login for a
   new identity at an origin does not remove credentials previously remembered
   for another identity at that origin. Expiry or a 401 therefore silently
   reauthenticates as the old identity. This can restore broader Docmost
   authority after the operator deliberately switched to a restricted identity.
2. **F-10 / IA-02 — release governance and immutable-input closure.** Live
   provider evidence shows the fork's `main` branch is unprotected, required
   status-check enforcement is off, and repository rulesets are empty. The
   release workflow does not bind its privileged job to a protected release
   environment, cryptographically verify a signed annotated tag, prove tag
   ancestry from the reviewed branch, or run/equivalently require the complete
   security gate. The Docker build also resolves unversioned apt packages from
   mutable repositories.

The controlled compatibility and Atlas runs were well-contained and their
cleanup evidence is internally consistent. They do not cure these source and
provider blockers. Any fix to source, workflow, lockfile, or build inputs creates
a new candidate and invalidates the current binary and live evidence.

## 1. Exact reviewed identities and evidence boundary

The assigned repository HEAD is an evidence-bearing integration snapshot:

| Identity | Exact value | Meaning |
| --- | --- | --- |
| Assigned review HEAD | `53c84ac4bb20ac0330a152d428f5a26186831e76` | Integrates compatibility and Atlas evidence |
| Assigned HEAD tree | `560f61f6304073622d78a617acd42a6030924db8` | Matches the Atlas predecessor's reported evidence tree |
| Hardened source candidate | `254b124ab89c6d5e3623ae99aa30583a2a43d632` | Exact source used for automated and live testing |
| Hardened source tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` | Source tree shared by candidate and tested integration |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` | Recalculated in this audit and equal at source candidate and assigned HEAD |
| Linux x86_64 test binary SHA-256 | `78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1` | Recalculated before compatibility/Atlas use and after Atlas cleanup |
| Compatibility evidence commit | `94ad7f6` | Adds sanitized Docmost v0.95.0 execution evidence only |
| Atlas evidence commit | `3f60b957d279107632f11c6088f29b8b2ba041ce` | Adds sanitized controlled-E2E evidence only |
| Atlas runtime commit/tree | `efad3719b67fc9949be3809a7d07b297a64de10d` / `58d8b8c5d330c905ef70b5be33b06883c0a57ae6` | Exact reviewed Atlas confirmation/probe runtime |

`git diff --name-status 254b124..53c84ac` contains only five files under
`verification/`; no Rust source, workflow, installer, configuration, dependency,
or lockfile drift occurred after the tested candidate.

The original accepted audit is the 2026-08-27 audit of baseline
`0bb296068227c9d2eb4e83731806867c2b0b98f6`. Its external assertions were not
treated as proof. This review instead inspected the current source, retained
full transcripts and their checksum manifest, repository-resident live records,
the predecessor harness, and fresh read-only provider responses.

## 2. Evidence inspected and safe checks

### Source and repository

- Complete diffs from baseline `0bb2960` to source candidate `254b124` and from
  candidate to assigned HEAD.
- Authentication, origin parsing, state/keyring storage, loopback server,
  HTTP policy, diagnostics, MCP routers and annotations, inputs/rendering,
  dependency policy, CI/release workflows, container, installer, configuration,
  tests, and operating documentation.
- `git status --short --branch`, commit/tree resolution, lockfile hashing,
  `git diff --check`, and `git diff --check 254b124..HEAD`.

### Automated and CI evidence

- `release-candidate-evidence.md` and every full transcript in predecessor
  `87e581c8-4fd2-4882-a245-01cc2d1155d8/verification-logs/`.
- Fresh `shasum -a 256 -c SHA256SUMS`: all ten retained evidence files matched.
- Both locked test transcripts report the complete suite passing in all-feature
  and no-default-feature modes; the candidate summary counts 131 tests and zero
  failures in each mode.
- Retained Clippy, format, release-integrity, pinned-input, and cargo-deny
  transcripts are successful. `cargo deny` reports `advisories ok, bans ok,
  licenses ok, sources ok` for the exact lockfile.
- Hosted GitHub Actions run `33162050785` records success for six platform
  builds, three Rust-check configurations, dependency policy, launcher smoke,
  release integrity, and the terminal security gate. The PR merge tree equals
  the candidate tree.
- Cargo was not installed in this card runtime; this audit therefore did not
  characterize retained Cargo transcripts as a fresh rerun.

### Live and provider evidence

- `verification/docmost-v0.95.0-compatibility-report.md` and its execution record.
- `verification/atlas-controlled-e2e-report.md`, its execution record, inspected
  configuration JSON, and the predecessor's `atlas-controlled-e2e-harness.py`.
- Fresh read-only GitHub repository, branch, ruleset, and release responses,
  reduced to non-secret facts in
  `verification/independent-remediation-audit-provider-evidence.json`.
- No provider mutation, release, merge, production credential use, production
  content access, or deployment action was performed by this audit.

## 3. Original-finding disposition

### F-01 — Saved credentials can cross origins

**Disposition: FIXED.**

Direct source establishes one canonical `(scheme, host, effective_port)` origin,
origin-scoped keyring identities and file names, origin fields on credentials and
sessions, process-lifetime origin pinning, and refusal to reuse legacy unscoped
state (`src/startup_config.rs:168-300`, `src/storage/keyring_store.rs:17-75`,
`src/storage/state_store.rs:66-133`, `src/auth/manager.rs:77-154,248-283`).

Negative tests reject ambiguous/non-HTTPS origins, refuse cross-origin state,
block an A-to-B login within one process, and prove a login redirect never
contacts the second origin (`tests/startup_config_test.rs`,
`tests/state_store_test.rs:144-203`, `tests/auth_flow_test.rs:88-196`). The live
candidate used one private HTTPS origin and origin-bound session state. No
credential-cross-origin bypass was found.

Residual risk: origin binding does not distinguish two identities at the same
origin. That separate defect is IA-01 under F-06.

### F-02 — Downloaded release executable is not integrity-verified

**Disposition: FIXED, with release-time residual conditions.**

The npm postinstall downloader now fails closed. The supported installer uses
HTTPS-only approved hosts, bounded manual redirects, same-filesystem staging,
Sigstore identity/issuer verification, exact expected commit/version/digest
agreement, artifact hashing, and final rename only after validation
(`npm/launcher/postinstall.js`, `scripts/install-atlas.sh:22-143`). The release
workflow emits a commit/lock/toolchain/artifact manifest, signs it, and requests
build provenance (`.github/workflows/release.yml:92-122`). Negative tests cover
digest mismatch, provenance rejection, partial transfer, oversize, unapproved
redirect, preservation of an existing executable, and staging cleanup.

The live binary was not fetched by npm: it was built from candidate `254b124`
with the locked command and pinned Rust container, then independently matched by
SHA-256 before compatibility, before Atlas use, and after cleanup. Atlas invoked
that absolute binary through literal SSH arguments without a shell or downloader.

Residual condition: no fork release exists, so no real tag, signed manifest,
published provenance, or production installer execution can yet be verified.
Publication and deployment remain separately unauthorized. IA-02 must be closed
before a first release can be accepted.

### F-03 — Ambient write authority and inaccurate annotations

**Disposition: FIXED.**

Default construction registers exactly ten read tools. Write construction
requires both explicit write mode and a nonempty exact allowlist, and filters the
write router to that set (`src/startup_config.rs:8-19,109-161`,
`src/server/tools.rs:20-38`). Invalid, duplicate, unknown, empty, and
programmatic bypass configurations fail closed. All ten mutations declare
`read_only_hint=false`, conservative non-idempotency, and destructive status
consistent with create/copy versus move/update semantics.

Automated tests enumerate every single-tool subset and annotation. Compatibility
showed ten reads by default, five exact writes in a separate process, and direct
`tool not found` results for a default write and an unallowlisted write. Atlas
showed ten reads by default and exactly `create_page` in a separate 11-tool
process. Approve-once produced one dispatch and one page; denial produced no
dispatch and no duplicate. The Atlas gate operated independently of hints.

Residual risk: Docmost permissions remain the ultimate server-side blast-radius
boundary. Production must use a restricted identity and transient narrow
allowlists.

### F-04 — Plain HTTP is accepted for secrets

**Disposition: FIXED.**

The canonical parser requires HTTPS except for explicit literal IPv4/IPv6
loopback development opt-in, rejects userinfo, queries, fragments, paths,
backslashes, unsupported schemes, hostname aliases, and non-loopback HTTP, and
normalizes default ports (`src/startup_config.rs:181-300`). Login and API clients
disable all redirects (`src/network_policy.rs:47-57`). Tests prove password and
bearer-token redirect targets are never contacted. Both live runs used the same
private HTTPS origin.

Residual risk: the loopback-HTTP override remains deliberately dangerous if an
operator enables it outside local testing; the literal-address restriction and
explicit flag bound that risk.

### F-05 — Vulnerable dependency baseline and no advisory gate

**Disposition: ACCEPTED WITH SPECIFIC EVIDENCE AND RESIDUAL RISK.**

The lockfile upgrades the originally identified active packages, removes the
native GTK/WebKit graph, and makes `native-webview` a no-op. CI runs cargo-deny
for advisories, bans, licenses, and sources, plus weekly scheduled checks and
Dependabot. The checksum-validated candidate transcript passes against the exact
lockfile and the hosted dependency job is successful.

The sole advisory ignore is `RUSTSEC-2026-0189` for `rmcp 0.6.4`. It is accepted
only because the supported application imports and constructs stdio transport
(`rmcp::transport::io::stdio`) and does not enable or construct the affected
Streamable HTTP server. `docs/dependency-policy.md:21-38` records the exact
feature-path rationale. This acceptance expires if the rmcp feature graph,
transport, version, or advisory scope changes.

Residual risk: advisory databases move. The retained scan is candidate-bound but
not a fresh scan from this card runtime. The remediation candidate must run a
fresh current cargo-deny scan and regenerate the exact dependency/feature-path
record; no new reachable high/critical advisory may be excepted.

### F-06 — Fallback encryption key is stored beside ciphertext

**Disposition: STILL BLOCKING.**

The original fallback defect is substantially repaired: keyring failures fail
closed by default, the weaker file mode requires explicit acknowledgement,
password persistence requires `remember_password=true`, origin-scoped fallback
files use restricted Unix modes, and `forget` removes scoped and legacy state
(`src/storage/state_store.rs:86-159,197-279`). Tests cover failure, opt-in,
cross-origin isolation, legacy non-reuse, and scoped idempotent forget.

However, `AuthManager::login` writes credentials only when the current input has
`remember_password=true`; the false branch does nothing
(`src/auth/manager.rs:211-219`). Existing remembered credentials for that origin
therefore survive a later session-only login. `reauthenticate` selects credentials
by origin and uses the stored email/password without verifying that the stored
identity matches the current configured identity (`src/auth/manager.rs:113-137`).

The demonstrated source sequence is:

1. Identity A logs into origin O with password persistence enabled.
2. Identity B later logs into the same O with `remember_password=false`.
3. B's config and session replace the active state, but A's credentials remain.
4. When B's session expires or receives 401, automatic reauthentication logs in
   as A and overwrites the active config/session.

If A has broader permissions, the connector silently regains them. No existing
test covers remembered-A to session-only-B rollover. This violates the runbook's
session-only and credential-rotation contract and blocks deployment.

### F-07 — Debug logging records private payloads

**Disposition: FIXED.**

Diagnostics use a positive field allowlist and discard unknown values, payloads,
URLs, origins, credentials, content, and raw errors (`src/debug.rs:5-69`). HTTP
transport errors are reduced to URL-free classes; bounded server bodies are
consumed but omitted from diagnostics (`src/network_policy.rs:85-97`,
`src/auth/manager.rs:403-411`). Canary tests cover secrets, content, errors,
origins, and redirects. Live repository evidence retains only synthetic labels,
counts, public commit identities, and sanitized outcomes. No leakage was found
in the reviewed report/config artifacts.

Residual risk: future debug call sites can still leak through the free-form
static message or scope parameters if developers interpolate untrusted data.
Current call sites use static strings; upstream-sync review must preserve that
property or make message/scope typed constants.

### F-08 — Missing deadlines and response limits

**Disposition: FIXED, with mandatory live retest.**

One non-tunable policy supplies 5-second connect and 30-second overall request
deadlines, no redirects, declared-length preflight, streaming response caps,
bounded error bodies, input/identifier/cursor/title/description/list limits, and
bounded rendered page output (`src/network_policy.rs`,
`src/docmost_client/mod.rs:57-79,319-449`,
`src/docmost_client/writes.rs:29-162`, `src/server/tools.rs:170-243`). Other list
and summary outputs are bounded by the 8 MiB response cap plus fixed rendered
item counts. No unbounded network read or result-count loop was found.

Negative tests exercise stalled login/API calls, chunked oversized success and
error bodies, authentication-body rejection before persistence, redirect refusal,
and boundary-plus-one inputs (`tests/network_safety_test.rs`).

Residual evidence gap: the controlling runbook requires timeout, redirect,
oversize, permission-denied, and server-error behavior to be observed through
the live Atlas path. The retained Atlas run exercised successful reads and one
confirmed write but none of these negative paths. Because IA-01/IA-02 will
create a new candidate, these cases must be included in the refreshed live run.

### F-09 — Loopback authentication hardening gaps

**Disposition: FIXED, with mandatory live retest.**

The server generates a 32-byte flow secret, binds IPv4 loopback on an ephemeral
port, validates exact Host, Origin, flow header, query secret, and JSON content
type, makes `/success` display-only, settles once, times out, and applies no-store,
no-referrer, nosniff, frame denial, and nonce-bound CSP headers
(`src/auth/local_server.rs:91-163,196-342,408-457`). The integrated test proves
wrong flow/Host/Origin/header requests fail and GET `/success` does not settle.

Residual evidence gap: live compatibility/Atlas evidence shows session-only
origin-bound state but does not retain an end-to-end browser-flow matrix or
origin-scoped `forget`/expiry cycle. Those runbook checks must be refreshed after
IA-01 is fixed.

### F-10 — Incomplete release provenance and repository controls

**Disposition: STILL BLOCKING.**

Positive source controls are real: Actions, Rust, and OCI base indexes are
pinned; builds use the lockfile; manifests bind commit, lock digest, toolchain,
artifact size/digest; cosign and GitHub provenance are configured; the Atlas
installer verifies identity and digest; upstream remains read-only; and the
operations guide requires full revalidation after every upstream sync.

Direct gaps remain:

- Fresh provider evidence reports `main` at baseline `0bb2960` with
  `protected=false`, required-status enforcement `off`, zero repository rulesets,
  `web_commit_signoff_required=false`, and an unsigned tip. This is direct
  evidence that the required reviewed-PR/check boundary is absent today.
- The privileged release job has `contents: write`, `id-token: write`, and
  `attestations: write` but no `environment:` binding
  (`.github/workflows/release.yml:63-71`). No protected release environment is
  evidenced.
- The workflow triggers on any `v*` tag and never checks that the tag commit is
  reachable from reviewed `main`, that the tag is annotated and
  cryptographically signed, or that the exact commit passed the complete CI
  security gate. `gh release create --verify-tag` is not retained cryptographic
  tag-signature evidence.
- The release job reruns only locked no-default-feature tests. It does not rerun
  or independently require format, Clippy, cargo-deny, installer negative tests,
  pinned-input checks, and the supported platform security matrix on that tag.
- Both Docker stages pin image digests, but `apt-get update && apt-get install`
  names packages without versions (`Dockerfile:16-28`). The resulting native
  libraries remain mutable build inputs and the static pinned-input test does not
  detect this.
- The public fork metadata still points its homepage to the upstream
  `@wisflux/docmost-local-mcp` npm package. That conflicts with the fork's
  no-upstream-downloader operating boundary and can misdirect operators.

There is no fork release, so signed tag/manifest/provenance output cannot yet be
verified. These gaps block the final independent gate and any release or
deployment decision.

## 4. Regression-risk review

| Control family | Current evidence | Regression assessment |
| --- | --- | --- |
| Credential and identity | Canonical origins, scoped state, keyring fail-closed, forget tests | **High / blocking:** same-origin remembered identity survives session-only identity change (IA-01). |
| Authority and annotations | Exact inventory/allowlist tests, compatibility inventory, Atlas approve/deny dispatch ledger | Low if router filtering and Atlas confirmation remain unchanged. Any new write must update the canonical inventory and annotation table. |
| Transport/origin | One shared no-redirect client, timeouts, body caps, strict URL parser, adversarial tests | Low in current source; live negative-path evidence must be refreshed on the remediated binary. |
| Logging/privacy | Positive diagnostic allowlist, content-free transport errors, sanitation scans | Low in current call sites; free-form message/scope arguments are an upstream-sync review hazard. |
| Dependencies | Exact lockfile, cargo-deny, no native webview graph, weekly automation | Medium accepted risk: moving advisory database and one exact unreachable rmcp exception. Refresh on every candidate/release. |
| Loopback auth | Random flow secret, exact request properties, headers, display-only success, single settlement | Low in current source; browser/forget/expiry live matrix remains to be recorded. |
| Supply chain/release | Signed-manifest design, fail-closed installer, digest-matched test binary | **High / blocking:** provider branch is unprotected; tag provenance and release gate are not closed; apt packages float. |
| Upstream sync | Read-only upstream policy and complete manual security checklist | Medium residual risk: procedural only. Require a recorded upstream range, full diff, invariant-by-invariant review, and full rerun for every sync. No automated sync may bypass review. |
| Compatibility | All ten reads plus five allowlisted writes exercised against isolated Docmost v0.95.0 | Low for the exact `254b124` binary; any source/lock/build-input change invalidates it. |
| Atlas operation | Exact absolute binary, ten-read inventory, one-tool write process, independent approve/deny result, cleanup | Strong for authority mechanics; incomplete for the runbook's full negative/lifecycle matrix and invalidated by required source changes. |
| Cleanup/non-production | Zero test containers/volumes/state/registrations; production container count remained three | Supported for the completed runs. Evidence proves bounded cleanup, not future cleanup correctness. |

## 5. Required remediation before the final independent gate

The successor must treat every item below as mandatory. It may not reclassify a
blocker without stronger direct evidence.

### IA-01 — Prevent same-origin remembered-identity rollback

1. On every successful `remember_password=false` login, remove any remembered
   credentials for that canonical origin before the login can be considered
   complete. Removal must cover keyring, origin-scoped fallback ciphertext/key,
   and relevant legacy credential state without deleting the newly written
   session/config.
2. Alternatively or additionally bind automatic credential reuse to both
   canonical origin and the currently configured identity, and fail closed on a
   mismatch. Do not silently reuse A when config/session identify B.
3. Make the multi-file transition failure-safe: if credential clearing or later
   persistence fails, do not leave a state that can silently roll back identity.
4. Add a regression test with exact sequence remembered A -> session-only B ->
   forced expiry/401. Assert A receives zero reauthentication requests, B is not
   silently replaced, and interactive authentication or an explicit safe error
   is required.
5. Add same-identity session-only rollover and idempotent-clear tests for both
   keyring and explicitly acknowledged fallback paths.

### IA-02 — Close release and provider governance

1. Protect `BonEvil/main`; require reviewed PRs and the exact terminal security
   checks; prevent force-push/deletion. Add a ruleset or branch protection and
   retain a fresh provider response proving it.
2. Protect release tags from update/deletion and require annotated,
   cryptographically signed tags. Add a workflow check that verifies the tag
   signature rather than only tag existence.
3. Require the tag commit to be reachable from the reviewed protected branch and
   to have a successful exact-commit security gate. Fail closed if either link is
   absent.
4. Bind the privileged release job to a protected release environment with
   required reviewer approval and least-privilege publication authority. Retain
   provider evidence of that environment.
5. Run or immutably require the complete security gate for the release commit,
   including format, Clippy, both feature-mode tests, current cargo-deny,
   release-integrity negatives, pinned inputs, and all supported platform builds.
6. Eliminate floating apt packages from security/release build inputs: pin exact
   versions/snapshot repositories or remove those packages from the artifact
   build path. Extend `check-pinned-inputs.sh` with a negative fixture that fails
   when unversioned apt inputs return.
7. Add release negative tests for wrong manifest commit/version, duplicate asset,
   malformed digest, unsigned/lightweight tag, tag outside protected ancestry,
   missing security-gate status, and absent environment approval.
8. Correct public fork metadata so it does not direct hardened-fork operators to
   the upstream npm package.

### IA-03 — Refresh invalidated automated and live evidence

After IA-01/IA-02, record one new exact source commit/tree, lockfile hash, binary
digest, and build-input identity. The current `254b124` binary evidence becomes
historical only.

1. Rerun format, Clippy with warnings denied, both locked test modes, a current
   cargo-deny scan, release-integrity negatives, pinned-input policy, manifest
   generation, workflow parsing, and all six platform builds.
2. Rebuild the Linux test binary from the exact new candidate and independently
   bind its digest to the source/tree/lock/toolchain.
3. Repeat isolated Docmost Community v0.95.0 compatibility with the restricted
   identity and synthetic disposable content; independently inspect results and
   cleanup.
4. Repeat controlled Atlas E2E with the new digest. Exercise all ten read tools
   through Atlas, prove all write names unavailable in default mode, then use the
   smallest separately approved allowlist and prove approval dispatches once and
   denial dispatches zero times.
5. Complete the runbook lifecycle/negative matrix: session expiry/401,
   same-origin identity change, origin-scoped forget, redirect, timeout,
   declared and chunked oversize, permission denial, and hostile server-error
   diagnostics. Retain only sanitized counts/classes and no private values.
6. Reconfirm zero temporary Atlas registrations, test containers/volumes,
   disposable state/secrets, and production changes, with exact bounded pre/post
   assertions.

## 6. Evidence integrity and deployment boundary

The repository contained no pre-existing uncommitted changes when review began.
This card changes only this report and the sanitized provider-evidence JSON. It
does not modify source, tests, workflows, configuration, provider state, Docmost,
Atlas registration, release state, or deployment state.

Deployment remains disabled. The current candidate must not proceed to the final
independent gate as ready, must not be released or installed for production, and
must not be merged. After every item above is resolved and freshly evidenced,
the final independent reviewer must reassess F-01 through F-10 and IA-01 through
IA-03 against the new exact candidate. Even a successful final gate authorizes
only the workflow's separately bounded repository delivery; merge, release, and
deployment still require their own authority.
