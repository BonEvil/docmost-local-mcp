# Independent-audit remediation ledger

Date: 2026-08-30

Deployment state: **DISABLED / BLOCKED**

This ledger maps the independent audit at `e96ae6e` to the exact repository-owned
remediation candidate and the subsequent provider-admin read-back supplied through
the resolved Atlas blocker. It does not treat historical live evidence as proof for
the new source identity and does not infer provider controls from workflow YAML.

## Exact identities

| Identity | Exact value |
| --- | --- |
| Remediated source commit | `f398ebb0ae253933f0ae4f1f891c2f19a3921aca` |
| Evidence commit / draft PR #2 head | `06bbe066686c3b0872c97d621e35015b6ab4291e` |
| Remediated source tree | `941f8f345ec54696a37816b5d7f9ec6b0d1d91aa` |
| `Cargo.lock` SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Debian snapshot policy SHA-256 | `805348a5d9a466214f8596eac0318be8e03b0269669f194a81c0834d3263e4c4` |
| Dockerfile SHA-256 | `8dc537b7581d311f74b5210a1de3d09e939326a07a9850bc280ac47ee05acc1e` |
| Linux arm64 candidate binary SHA-256 | `e04679739a62be7ce9db71c2f8dd47ed22d058ffd9bc4da3782ab7aa21542f5f` |
| Isolated-host Linux x64 candidate binary SHA-256 | `a7318362aac167aead3c008ff43ab11ec9f7efdf0953e7ad4ce5a7975338c291` |
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
| F-08 / IA-03 | Timeout, redirect, declared/chunked oversize, and safe-error automated regressions pass on `f398ebb`. | Source checks pass; mandatory refreshed live Atlas negative matrix is blocked | The isolated stack exists, but no Docmost registration or secure launch coordinates are available to this resumed Card Run; no claim is made that these paths ran through Atlas on the x64 binary. |
| F-09 / IA-03 | Loopback secret/request/header and lifecycle regressions pass on `f398ebb`; session identity rollback regression is added. | Source checks pass; mandatory refreshed browser/forget/expiry live matrix is blocked | The restricted identity exists, but the absent Docmost registration prevents the mandatory Atlas lifecycle matrix. |
| F-10 / IA-02 | Release now accepts only provider-verified annotated signed tags whose exact commit is in `origin/main` and has exactly one successful terminal security gate; privileged publication is bound to `protected-release`. Negative tests reject lightweight/unsigned tags, outside ancestry, and missing/wrong gate status. Debian repos are dated snapshots and direct apt packages have exact versions. Fork-owned Cargo/npm metadata is explicit. Provider-admin read-back confirms the single-maintainer controls detailed below, and all 13 hosted checks on exact evidence commit `06bbe06` succeeded. | Fixed under the authorized single-maintainer governance model | The zero-human-approval model depends on the exact terminal gate, enforced admins, immutable tag/release controls, and protected-release approval remaining configured. A real release remains intentionally unauthorized and untested. |

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

Draft PR #2 contains exact evidence commit `06bbe06`. A fresh provider read through
the bound Atlas repository connection confirmed the PR is open, draft, unmerged,
and points to `main`; all 13 hosted check runs completed successfully. Those runs
include six platform builds, three Rust checks, dependency policy, launcher smoke,
release integrity, and the exact terminal `Integrated security and release gate`.
No merge, tag, release, or deployment was performed.

## Provider-control resolution

The earlier audit-time provider snapshot remains preserved separately and is not
rewritten. The resolved blocker supplied fresh administrator read-back for the
authorized single-maintainer model, recorded in
`verification/audit-remediation-provider-evidence.json`:

- `main` requires pull requests with zero human approvals plus the strict exact
  check `Integrated security and release gate`; admins are enforced.
- Stale-review dismissal, conversation resolution, and linear history are enabled;
  force-push and deletion are disabled.
- Active ruleset `21860233` protects `refs/tags/v*` from update and deletion with
  no bypass, and immutable releases are enabled.
- `protected-release` requires BonEvil approval with self-review permitted for the
  single-maintainer model and limits deployment to protected branches.
- Actions allow only the nine reviewed full-SHA references; broad GitHub-owned and
  verified-creator allowances are disabled.
- The public homepage points to the BonEvil fork.

## Remaining genuine external blocker and exact clearance evidence

The resumed blocker resolution confirms the isolated Docmost Community v0.95.0
stack, restricted identity, disposable space, protected host-side session state,
and exact Linux x64 binary exist. Its bounded preflight proved protocol
`2025-03-26`, exactly ten read-only tools, and successful `get_current_user` on
binary digest `a7318362…`. The ephemeral password remains only in the approved
host-side mode-0600 file and was never printed, copied, or persisted by this card.

However, the resumed Card Run contains no Docmost MCP registration: a fresh Atlas
server-list read returned only the three unrelated enabled MCP servers. Neither the
card workspace nor the supplied resolution contains the approved SSH target or
private origin needed to recreate the sanitized launch arguments. The registered
browser fallback also could not reach the loopback Atlas UI, while the loopback API
confirmed the registration is absent. Reading private SSH/Tailscale configuration
or guessing a host would exceed the accepted scope. Therefore the complete
all-ten-read/default-write-denial, approve-once/deny-zero, identity/forget/expiry,
redirect/timeout/oversize, permission/server-error, sanitation, and cleanup matrix
still cannot be refreshed safely. The earlier `254b124` live evidence and the new
bounded preflight are not substituted for that mandatory matrix.

Clearance requires either re-registering the already-approved isolated Docmost MCP
with this Card Run or supplying the exact approved SSH target and private origin as
secure execution-time inputs. The complete matrix must then run through Atlas on
Linux x64 digest `a7318362…`, followed by sanitation and cleanup verification. Any
source, lockfile, or build-input change requires a new identity and another complete
rerun.

No merge, tag, release, production credential/content access, Atlas production
registration, production change, or deployment occurred. Production remained the
reported three containers and was not accessed. Deployment remains disabled.
