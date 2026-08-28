# Operations and maintenance

This is the controlling runbook for the hardened fork. The repository checks
produce a release candidate; they do not authorize publication, installation,
deployment, or write access. Deployment stays disabled until the live Docmost
v0.95.0, Atlas end-to-end, provider-control, and independent-review gates below
are recorded as passed for the exact candidate commit and lockfile.

## Credential and session lifecycle

Use HTTPS to a canonical Docmost origin. Plain HTTP is allowed only for a
literal loopback address and only with the explicit development flag. Use a
restricted Docmost identity whose permissions are no broader than the required
read operations. Do not enable remembered passwords unless unattended
reauthentication is required and the operating-system keyring is available.
Passwords are session-only by default. The encrypted-file fallback is weaker,
disabled by default, and may be used only after explicit acknowledgement.

Run `docmost-local-mcp forget --base-url <canonical-origin>` when rotating the
identity, changing the canonical origin, ending an operator session, or
suspecting session exposure. Confirm the origin-scoped session and credentials
are gone before re-enrollment. The complete deletion and loopback matrices are
in [Credential and loopback authentication lifecycle](credential-auth-lifecycle.md).

Never put a password, auth token, cookie, private page content, user email, or
real origin into command transcripts, CI artifacts, issue text, or release
evidence. Debug output is metadata-only, but evidence must still be inspected
before retention.

## Secure Atlas operation

1. Install only a reviewed, signed fork binary using the procedure in
   [Atlas release integrity](atlas-release-integrity.md). Use an absolute path
   owned by the Atlas service account and not writable by untrusted users.
2. Start from `config/atlas-mcp.production.example.json`: HTTPS canonical
   origin, absolute binary, `read-only` authority, no credential-file fallback,
   and no loopback-HTTP exception.
3. Keep Atlas confirmation mandatory for every mutation. MCP annotations are
   descriptive and never replace Atlas authorization.
4. If writes are temporarily necessary, approve the task first, use a
   restricted identity and disposable Docmost space, enable `write` mode with
   the smallest exact allowlist, then restore read-only configuration and
   restart the MCP process when the task ends.
5. Do not use the npm launcher, a moving branch, an unverified existing binary,
   or a shell wrapper as the Atlas command.

## Repository candidate gate

Run from a clean checkout of the exact candidate using Rust 1.98.0:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
cargo deny check advisories bans licenses sources
bash tests/release_integrity_test.sh
bash scripts/check-pinned-inputs.sh
git diff --check
```

Record the full `git rev-parse HEAD`, `sha256sum Cargo.lock`, complete command
outputs, runner OS/architecture, Rust/Cargo versions, and a clean `git status`.
Compare the passing tests to
[Security-invariant coverage](../verification/security-invariant-coverage.md).
Any failure closes the gate; do not weaken a control or add an exception merely
to obtain green output.

## Required live validation

Live validation requires Docmost Community v0.95.0, a dedicated restricted
test identity, a disposable space containing synthetic non-private content,
and an approved credential mechanism: interactive session-only authentication
or an OS keyring. Do not use the encrypted-file fallback for this gate.

With Atlas still isolated from production workloads:

1. Verify the server reports v0.95.0 and authenticate to the exact canonical
   HTTPS origin. Exercise all ten read tools through Atlas in read-only mode.
2. Confirm the Atlas inventory contains no write tools and that direct requests
   for each write name fail as unavailable.
3. After separate write-test approval, restart with only the write tools needed
   for the disposable space. Confirm Atlas asks for independent confirmation.
   Exercise each enabled mutation, verify the Docmost result, and verify an
   unallowlisted mutation remains unavailable.
4. Exercise session expiry/relogin and origin-scoped forget. Confirm redirect,
   timeout, oversized-input, permission-denied, and server-error failures do not
   disclose credentials, content, or origins in retained diagnostics.
5. Restore read-only configuration, restart, and re-check the ten-tool inventory.

Retain only sanitized results tied to the exact candidate. A passing live test
does not itself approve deployment.

## Upstream synchronization

Treat upstream as read-only. Fetch it, create a fork branch from the last
reviewed fork commit, and inspect the complete upstream diff before applying
changes. Merge or cherry-pick without rewriting fork history. Resolve conflicts
in favor of the fork controls unless a replacement is demonstrated to be at
least as strict.

Every sync must explicitly review authentication and canonical-origin parsing,
credential/session storage and deletion, loopback flow protections, HTTP client
timeouts/redirects/body bounds, diagnostic redaction, MCP authority/annotations,
dependency policy, workflows, installer, manifest/signing, and Atlas config.
Run the full repository and live gates again. Do not accept an upstream change
that reintroduces native-webview dependencies, production npm download, broad
writes, redirect following, unbounded bodies, payload logging, floating build
inputs, or a credential fallback by default.

Preserve the upstream MIT license and attribution. Record upstream refs,
reviewed commit range, conflicts, security-sensitive decisions, test evidence,
and reviewer approval.

## Security patches, dependencies, and exceptions

Dependabot and scheduled CI are triage inputs, not automatic merge authority.
For each security or dependency update, inspect the advisory and affected
feature path, update in an isolated branch, review `Cargo.lock`, run both feature
configurations and the release-integrity checks, then repeat live validation if
runtime behavior can change.

An advisory exception must be exact, time-bounded by active review, tied to a
locked dependency path, and supported by reproducible reachability evidence.
Document the owner, rationale, affected versions/features, compensating
controls, removal condition, and next-review date. High or critical reachable
advisories have no release exception. Review the existing `rmcp` exception on
every `rmcp` update and at every release; remove it when fixed or if Streamable
HTTP becomes reachable. See [Dependency policy](dependency-policy.md).

For an urgent fork security patch, preserve all existing controls, add a
regression test that fails before the patch, run the full gate, obtain review,
and issue a new signed release candidate. Never replace an existing artifact or
tag. Rotate affected credentials and forget sessions when the defect could have
exposed them.

## Release and rollback

After repository checks, live v0.95.0 validation, Atlas end-to-end validation,
provider-control inspection, and an independent security review all pass on the
same commit, a maintainer may separately authorize the tag/release workflow.
The release must contain the versioned binaries, `SHA256SUMS`, commit-bound
manifest, Sigstore bundle, and GitHub provenance described in the release
integrity document. Verify provider-side protections before publication.

Installation or deployment needs a separate approval. Record reviewed commit,
tag, manifest digest, binary digest, destination, and installer output. Keep the
last verified binary and configuration available for rollback. On rollback,
atomically reinstall that separately verified artifact, restore its compatible
read-only configuration, restart Atlas, verify the ten-tool inventory, and
forget/rotate sessions if compromise is possible. Never roll back by downloading
through npm or selecting a moving branch.
