# Docmost Community v0.95.0 compatibility report

> Superseded for v0.9.4 by
> [`v0.9.4-refresh-evidence-r2.md`](v0.9.4-refresh-evidence-r2.md). The record
> below remains the historical v0.9.3 exact-candidate result.

**Status:** passed in the isolated disposable compatibility boundary on
2026-08-31. This report replaces the stale candidate-bound August 28 result.
It is sanitized: no endpoint, credential, token, cookie, session filename,
private identifier, private content, or sensitive response value is retained.

The command and observed-output record is
[`docmost-v0.95.0-live-evidence.md`](docmost-v0.95.0-live-evidence.md).

## Exact tested candidate

| Field | Value |
| --- | --- |
| Workflow integration commit | `b93024c6094abe5044b56f32991e39d650c6c9e5` |
| Inherited v0.9.3 candidate commit | `a33895bef1c66c3bb0855c4d2ee06cef252c020f` |
| Shared tested source tree | `327d122cf9695b586e6999142a101b86bda4f67a` |
| `Cargo.lock` SHA-256 | `3f2a639c0bed73088017f70fe9564a7d1696c24b4884e51bccf20284ce4754b9` |
| Linux x86-64 binary SHA-256 | `7585777a8423f0b4c867331c31f250cf3aa7b0fef4f38a41c88e89287f66cd52` |
| Build image | `rust:1.98.0-slim-bookworm@sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157` |
| Build command | `cargo build --locked --release --no-default-features` |
| Docmost image | `docmost/docmost:0.95.0@sha256:41c8d777cf23c74e78f94e676aec328b7d7856f48df5e573543dac68d371e37c` |

The integration and inherited candidate commits resolve to the same source
tree. The binary was built from a read-only archive of that exact tree in the
repository-pinned Rust image on the accepted Ubuntu x86-64 host. Its digest was
calculated after build, immediately before cleanup, and did not change.

No source, lockfile, build-input, runtime, or policy correction was made by this
card. This card changes only these two refreshed evidence records. The current
branch also mechanically integrated a sibling card's two Atlas-control evidence
records after the first review; those files likewise change no source, lockfile,
or build input. No source correction exists for a downstream gate to retest.

## Bounded environment

The disposable Compose project contained exactly three containers and three
project-labelled volumes:

```text
docmost/docmost:0.95.0
postgres:16-alpine
redis:7.2-alpine
```

It bound only the previously unused loopback port behind the already-existing
private HTTPS `:8443` route. The route was read, not changed. The ordinary
Docmost stack remained a distinct three-container project behind loopback port
`3001`; no ordinary endpoint, page, space, identity, database, volume,
credential, configuration, or response body was accessed.

The test workspace, identity, space, pages, bodies, and comment were synthetic.
The password existed only in a host-side mode-`0600` file inside the disposable
directory. Authentication state was session-only; the state directory was
`0700`, its two state files were `0600`, and no credential file existed.

## Complete compatibility matrix

| Phase | Required case | Result | Independent evidence |
| --- | --- | --- | --- |
| Server | Confirm Docmost Community version | Pass | The package inside the digest-pinned container reported `0.95.0`. |
| Protocol | Initialize exact candidate | Pass | MCP negotiated `2025-03-26`. |
| Read-only authority | Enumerate default tools and annotations | Pass | Exactly ten tools, every one `readOnlyHint: true`, and zero write tools. |
| Read-only authority | Directly request every mutation name | Pass | All ten mutation names returned `code=-32602`, `tool not found`. |
| Initial reads | Current user, workspace members, spaces, space details, pages, and both searches | Pass | Each call completed in the synthetic one-user/one-space boundary. |
| Write authority | Start a separate smallest allowlisted process | Pass | Exactly ten reads plus `create_page`, `update_page`, `move_page`, `create_comment`, and `update_comment`; all five writes were annotated non-read-only. |
| Write exclusion | Request every unallowlisted mutation | Pass | `duplicate_page`, `copy_page_to_space`, `move_page_to_space`, `create_space`, and `update_space` each returned `tool not found`. |
| Synthetic pages | Create parent and Markdown child | Pass | Direct isolated database inspection found exactly two synthetic pages. |
| Page update | Update child title and body | Pass | Fresh read-only `get_page` and direct database inspection both observed the update. |
| Page move | Nest child beneath parent | Pass | Fresh `list_child_pages` and direct database inspection both found one nested child. |
| Comment | Create and update one comment | Pass | Fresh `get_comments` found one comment; direct database inspection observed the updated body. |
| Final reads | Exercise `get_page`, `list_child_pages`, and `get_comments` in a new default process | Pass | These completed the ten-read matrix and independently confirmed the write result. |
| Restore authority | Re-enumerate a fresh default process | Pass | Inventory returned to the exact ten reads and no writes. |
| Session lifecycle | Force expiry, require interaction, then establish a fresh session-only login | Pass | Expired state was not silently reused; fresh login restored a successful read. |
| Forget lifecycle | Forget the exact canonical origin | Pass | The command exited zero and the state directory contained zero entries. |
| Redirect diagnostic | Return a redirect from a dedicated hostile loopback origin | Pass | The candidate returned the safe redirect status; the redirect target received zero requests. |
| Timeout diagnostic | Stall a dedicated hostile loopback origin beyond the production deadline | Pass | The candidate failed closed at `30.0` seconds and retained no origin or canary. |
| Oversized input | Submit a search query one byte above the production limit | Pass | The candidate rejected it before network dispatch; the hostile origin received zero requests. |
| Oversized responses | Return declared-length and chunked bodies above the production limit | Pass | Both were rejected at the `8388608`-byte cap with no retained response content. |
| Permission diagnostic | Return `403` with a 718-byte hostile body | Pass | The body was omitted; no canary, origin, URL, or response excerpt escaped. |
| Server-error diagnostic | Return `500` with a 718-byte hostile body | Pass | The body was omitted; no canary, origin, URL, or response excerpt escaped. |

All ten reads were exercised: `list_workspace_members`, `get_current_user`,
`search_docs`, `list_pages`, `get_comments`, `list_child_pages`, `search_pages`,
`list_spaces`, `get_space`, and `get_page`.

The six successful allowlisted mutation calls were two `create_page` calls and
one each of `update_page`, `move_page`, `create_comment`, and `update_comment`.
Only identifiers returned by the isolated instance were used.

## Independent result inspection

A fresh default process, distinct from the write process, observed the updated
page title and body, one nested child, and one comment. A direct read-only SQL
inspection of only the disposable database returned:

```text
synthetic_pages=2
nested_child=1
updated_page_body=1
synthetic_comments=1
updated_comment_body=1
```

## Ordinary-service invariance and cleanup

Before creation, during the test, and after cleanup, hashes of the ordinary
container inventory, ordinary container IDs/start times, and complete Tailscale
Serve configuration were identical. The ordinary `:3001` listener remained
present. The stale `:8443` route was left intact exactly as required.

Cleanup removed the three disposable containers, three volumes, project
network, synthetic database/content, disposable runtime home, password,
session files, source archive, Cargo home, test harness, and tested binary.
Post-cleanup inspection returned zero compatibility-labelled containers,
volumes, and networks; the disposable loopback listener was absent; both the
remote disposable directory and local build/harness artifacts were absent.

The corrective hostile-loopback run used the same deterministic candidate
binary digest. It also removed its source export, Cargo home, build target,
binary, harness, temporary homes, and build/cleanup containers; post-cleanup
inspection found all of them absent.

This result establishes isolated Docmost Community v0.95.0 compatibility for
the exact tested source tree. It does not authorize Atlas control testing,
release, installation, activation, or ordinary-service access.
