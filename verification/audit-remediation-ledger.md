# Independent-audit remediation ledger

Date: 2026-08-30

Deployment state: **DISABLED / BLOCKED**

This ledger maps the independent audit at `e96ae6e` to the exact repository-owned
remediation candidate. It does not treat historical live evidence as proof for the
new source identity and does not infer provider controls from workflow YAML.

## Exact identities

| Identity | Exact value |
| --- | --- |
| Remediated source commit | `f398ebb0ae253933f0ae4f1f891c2f19a3921aca` |
| Remediated source tree | `941f8f345ec54696a37816b5d7f9ec6b0d1d91aa` |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Debian snapshot policy SHA-256 | `805348a5d9a466214f8596eac0318be8e03b0269669f194a81c0834d3263e4c4` |
| Dockerfile SHA-256 | `8dc537b7581d311f74b5210a1de3d09e939326a07a9850bc280ac47ee05acc1e` |
| Linux arm64 candidate binary SHA-256 | `e04679739a62be7ce9db71c2f8dd47ed22d058ffd9bc4da3782ab7aa21542f5f` |
| Local OCI image identity | `sha256:fd3a6ba9a80eda4deb08b69c1480a289991232751c1bb03a74b07c2d8306fa17` |
| Generated manifest SHA-256 | `72c9fcefee6f69aaed1e57be6491bffd6a4154f9f83742a1ecacad03632b6140` |

The generated binary and manifest are retained in the card workspace under
`artifacts/f398ebb/`. They are test artifacts only, not published release assets.

## Finding traceability

| Audit item | Change and test/evidence | Current disposition | Residual risk |
| --- | --- | --- | --- |
| F-01 | Canonical-origin controls are unchanged. Both locked suites and origin/auth regressions pass on `f398ebb`. | Fixed in source | Same-origin identity risk is handled separately by IA-01. |
| F-02 | Installer integrity controls remain; wrong commit/version, duplicate asset, malformed digest, digest mismatch, provenance failure, partial/oversize transfer, and unapproved redirect all reject while preserving the prior executable. | Fixed in source | A real signed fork release remains unauthorized and untested. |
| F-03 | Authority inventory/allowlist and mutation annotations are unchanged; both locked suites pass. | Fixed in source | Atlas remains an independent mandatory mutation gate. |
| F-04 | Canonical HTTPS/explicit literal-loopback and no-redirect controls are unchanged; redirect regressions pass. | Fixed in source | Loopback HTTP remains development-only. |
| F-05 | `cargo-deny 0.20.2` ran against the exact unchanged lockfile and current RustSec database: advisories, bans, licenses, and sources all passed. | Accepted with existing exact `rmcp` exception | Moving advisory data and the unreachable transport exception require release-time review. |
| F-06 / IA-01 | Session-only login now clears scoped keyring, fallback ciphertext/key, and relevant legacy credentials before config/session commit. Reauthentication independently compares remembered and configured email, clears mismatches, and fails before network dispatch. Tests cover remembered A -> session-only B -> forced 401, stale-credential reinsertion, same-origin/session preservation, fallback idempotency, and keyring-delete idempotency. | Fixed in source | Interactive re-enrollment is intentionally required after session-only expiry. |
| F-07 | Positive diagnostic allowlist and transport-error reduction are unchanged; full suites pass. | Fixed in source | Future free-form diagnostic call sites remain review-sensitive. |
| F-08 / IA-03 | Timeout, redirect, declared/chunked oversize, and safe-error automated regressions pass on `f398ebb`. | Source checks pass; mandatory refreshed live Atlas negative matrix is blocked | No claim is made that these paths ran through live Atlas on this binary. |
| F-09 / IA-03 | Loopback secret/request/header and lifecycle regressions pass on `f398ebb`; session identity rollback regression is added. | Source checks pass; mandatory refreshed browser/forget/expiry live matrix is blocked | No restricted live identity or isolated v0.95.0 stack is available in this card runtime. |
| F-10 / IA-02 | Release now accepts only provider-verified annotated signed tags whose exact commit is in `origin/main` and has exactly one successful terminal security gate; privileged publication is bound to `protected-release`. Negative tests reject lightweight/unsigned tags, outside ancestry, and missing/wrong gate status. Debian repos are dated snapshots and direct apt packages have exact versions. Fork-owned Cargo/npm metadata is explicit. | Repository-controlled remediation complete; provider enforcement remains blocking | GitHub `main`/`v*` protections, required checks, immutable releases, protected environment reviewers, and public homepage require provider-admin changes plus read-back evidence. |

## Fresh checks on the exact source candidate

All commands below completed successfully against `f398ebb` unless explicitly
identified as externally blocked:

- `cargo fmt --check` in pinned Rust 1.98.0 container.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`.
- `cargo test --locked --all-features`: 135 tests, zero failures.
- `cargo test --locked --no-default-features`: 135 tests, zero failures.
- `cargo deny check advisories bans licenses sources` with cargo-deny 0.20.2:
  advisories/bans/licenses/sources all `ok` (duplicate-version diagnostics are warnings).
- `bash tests/release_integrity_test.sh`.
- `bash tests/release_context_test.sh`.
- `bash scripts/check-pinned-inputs.sh`.
- YAML parsing of CI and release workflows; expected jobs were present.
- `git diff --check` and clean source-candidate worktree.
- Full multi-stage Docker build using the exact OCI digests, dated Debian snapshot,
  and exact requested apt versions.
- Commit-bound manifest generation and independent SHA-256 verification.

Six hosted platform builds and the exact-commit hosted terminal gate cannot run
until this candidate is published into the provider review path. Publishing a
branch or opening a pull request is outside this card.

## Genuine external blockers and exact clearance evidence

The independent audit's sanitized provider read at 2026-08-30T13:04:12Z reported
`main` unprotected, required status enforcement off, zero rulesets, an unsigned
tip, no fork releases, and no evidenced protected release environment. This card
has no authorized provider-settings operation, and repository inspection found no
durable provider binding for the assigned worktree. Source YAML cannot replace
provider enforcement.

No Docmost v0.95.0 test container/process, Atlas-controlled Docmost MCP
registration, dedicated restricted identity, or disposable test space is present
in this runtime. Therefore the compatibility, all-ten-read/default-write-denial,
approve-once/deny-zero, identity/forget/expiry, redirect/timeout/oversize,
permission/server-error, sanitation, and cleanup matrices cannot be refreshed
safely. The earlier `254b124` live binary evidence is historical only.

Clearance requires an authorized administrator to configure and provide fresh
read-back evidence for the GitHub controls above, and an authorized isolated
execution environment to run the complete Docmost v0.95.0 plus Atlas matrix on
binary digest `e0467973…`. Any source, lockfile, or build-input change requires a
new identity and another complete rerun.

No push, pull request, merge, tag, release, provider mutation, production
credential/content access, Atlas production registration, installation, or
deployment occurred. Deployment remains disabled.
