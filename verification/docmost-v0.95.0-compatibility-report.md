# Docmost Community v0.95.0 compatibility report

**Status:** blocked before live execution. This is a sanitized prerequisite and
provenance record, not a compatibility pass.

## Candidate identity

| Field | Value |
| --- | --- |
| Integration commit | `ec295b1d0ba69899ae8b381599f32833cbb25b8d` |
| Hardened candidate commit | `254b124ab89c6d5e3623ae99aa30583a2a43d632` |
| Candidate source tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Integration source tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Cargo.lock SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Binary digest | Not available: no supplied verified binary and this Card Run has no Cargo toolchain. |

The candidate and integration commits have the same source tree. No source
change was made by this card.

## Secure live-test prerequisites

The assigned card workspace and worktree were inspected for an approved
execution-time Docmost access configuration without reading credential values.
No approved restricted-identity session, OS-keyring entry, canonical target
reference, or disposable-space authority was supplied. The environment also
has no `DOCMOST_*` execution configuration.

The required secure mechanism is therefore unavailable. This report contains
no URL, credential, token, cookie, private content, or sensitive identifier.

## Operation record

| Scope | Required check | Result |
| --- | --- | --- |
| Server | Confirm Community v0.95.0 at the canonical HTTPS origin | Not run: no approved target reference or authenticated restricted identity. |
| Candidate | Run the exact hardened binary and record SHA-256 | Not run: no supplied verified binary; no Cargo toolchain in the card environment. |
| Read-only | Run representative bounded reads with no write tools exposed | Not run: live access unavailable. |
| Allowlisted writes | Create/update/move/comment only in the disposable space | Not run: disposable-space authority unavailable. |
| Independent inspection | Confirm intended disposable results in Docmost | Not run: live access unavailable. |
| Production scope | Compare bounded production inventory before/after | Not run; absence of production modifications is not claimed. |
| Cleanup | Remove only disposable artifacts and record the result | Not applicable: no disposable artifacts were created. |

## Local evidence and remaining blockers

The source provenance and lockfile digest above were independently checked in
the assigned worktree. The dependency handoff records successful local and
hosted regression coverage for candidate `254b124`, but those checks do not
substitute for this live compatibility gate.

Completion requires an approved secure mechanism that provides all of the
following at execution time: a canonical Docmost Community v0.95.0 HTTPS
target, a restricted test identity authenticated through an interactive
session-only flow or OS keyring, a disposable synthetic-content space, and the
verified candidate binary with its expected SHA-256. The test must then be run
in read-only mode first, followed by the smallest explicit write-tool
allowlist, with independent Docmost inspection and bounded cleanup.
