# Docmost Community v0.95.0 compatibility report

**Status:** passed within the isolated disposable compatibility boundary on
2026-08-28. This report is sanitized: it omits URLs, credentials, tokens,
cookies, private identifiers, page IDs, and private content.

The corresponding command and observed-output record is
[`docmost-v0.95.0-live-evidence.md`](docmost-v0.95.0-live-evidence.md).

## Candidate identity

| Field | Value |
| --- | --- |
| Integration commit | `ec295b1d0ba69899ae8b381599f32833cbb25b8d` |
| Hardened candidate commit | `254b124ab89c6d5e3623ae99aa30583a2a43d632` |
| Candidate source tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Integration source tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Cargo.lock SHA-256 | `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690` |
| Tested binary SHA-256 | `78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1` |
| Tested server | Separate `docmost/docmost:0.95.0` container stack over private HTTPS |

The remote binary digest was independently recalculated before testing and
matched the supplied expected digest. The candidate and integration commits
have the same source tree. No source change was made by this card.

## Authority and containment evidence

- The test identity was the only member of a one-member disposable workspace
  in the isolated instance.
- The remote session state was origin-bound and had directory mode `0700` and
  file modes `0600`. No credential value or session material was read.
- Read calls used a default read-only process. Its `tools/list` response
  contained exactly the ten read tools and no mutation tool; a direct
  `create_page` request returned `tool not found`.
- The write process exposed the ten read tools plus exactly five explicitly
  allowlisted mutations: `create_page`, `update_page`, `move_page`,
  `create_comment`, and `update_comment`. An unallowlisted `create_space`
  request returned `tool not found`.

## Sanitized operation matrix

| Phase | Bounded operation | Result | Independent confirmation |
| --- | --- | --- | --- |
| Read-only | Initialize and enumerate tools | Pass: ten read tools only | Separate later read-only process again enumerated no writes. |
| Read-only | Current-user, workspace-member, space, page-list, and both search tools | Pass: restricted one-member disposable scope; empty initial space; bounded no-result searches | Returned solely isolated-instance data. |
| Write | Create a synthetic parent page and a synthetic Markdown child page | Pass | Direct isolated Docmost database inspection found two synthetic pages. |
| Write | Update the synthetic child title and body | Pass | Fresh read-only `get_page` returned updated synthetic state. |
| Write | Move the child beneath the parent | Pass | Fresh `list_child_pages` and direct database inspection both found one nested child. |
| Write | Create and update one synthetic comment | Pass | Fresh `get_comments` and direct database inspection both found one comment. |
| Read-only | `get_page`, `list_child_pages`, and `get_comments` after writes | Pass | A separately launched default read-only binary process returned the expected synthetic state. |

All ten read tools were exercised across the initial and post-write read-only
phases: `list_workspace_members`, `get_current_user`, `search_docs`,
`list_pages`, `get_comments`, `list_child_pages`, `search_pages`,
`list_spaces`, `get_space`, and `get_page`.

## Production non-modification and cleanup

All test traffic and direct inspection were constrained to the separate
compatibility Compose project and its private HTTPS origin. No ordinary
production page, space, identity, database, endpoint, or container was
targeted or read. Before cleanup, the compatibility project contained exactly
three containers and three project-labelled volumes; the distinct production
project contained three containers.

After post-write independent inspection, cleanup removed only those exact three
compatibility containers and three compatibility-labelled volumes. A post-check
found zero compatibility containers and volumes, while the distinct production
project still had its three original containers. This is bounded evidence that
the disposable artifacts were removed and this card did not modify ordinary
production Docmost content.

## Failures and remaining blockers

No compatibility operation failed. The previous missing-access blocker was
resolved using the approved private isolated boundary. There are no remaining
compatibility blockers; this successful isolated test does not authorize
release, deployment, or access to the ordinary production instance.
