# Sanitized Atlas controlled end-to-end validation report

**Status:** passed on 2026-08-30 within the isolated disposable Docmost
Community v0.95.0 boundary. Production deployment remains disabled.

The corresponding sanitized command/result record is
[`atlas-controlled-e2e-live-evidence.md`](atlas-controlled-e2e-live-evidence.md),
and the inspected non-secret configuration is
[`atlas-e2e-inspected-config.json`](atlas-e2e-inspected-config.json).

## Candidate and runtime identity

| Field | Exact value |
| --- | --- |
| Hardened fork commit | `254b124ab89c6d5e3623ae99aa30583a2a43d632` |
| Hardened fork tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Reviewed Linux x86_64 binary SHA-256 | `78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1` |
| Atlas confirmation/probe commit | `efad3719b67fc9949be3809a7d07b297a64de10d` |
| Atlas confirmation/probe tree | `58d8b8c5d330c905ef70b5be33b06883c0a57ae6` |
| Negotiated MCP protocol | `2025-03-26` |
| Docmost version | Community `0.95.0` |

Atlas executed the exact reviewed remote fork binary by absolute path through a
literal SSH stdio command. The digest was recalculated before execution and
after cleanup. Neither launch used the retained npm launcher, `npx`, an upstream
downloader, a shell wrapper, a moving branch, or an unverified binary.

## Read-only authority result

The default process exposed exactly ten tools, all read-only:

```text
get_comments
get_current_user
get_page
get_space
list_child_pages
list_pages
list_spaces
list_workspace_members
search_docs
search_pages
```

No write-capable tool was present. Through separate default-authority Atlas
runtimes, `get_current_user`, `list_spaces`, and space-scoped `list_pages` each
returned a non-error MCP result. Atlas held each untrusted generic call before
dispatch, consumed an exact approve-once decision, and recorded
`dispatch_succeeded`. No returned identity, space, page, URL, or content value
was retained in evidence.

## Explicit write authority and independent confirmation

The write phase used a separate process with explicit write mode and the exact
one-tool allowlist `create_page`. Its inventory was the ten reads plus only
`create_page`; `create_space` and all other mutations were unavailable.

Atlas's gate operated independently of MCP annotations:

- the future remained blocked before a decision;
- the confirmation projection omitted the supplied identifier and synthetic
  argument values;
- approve-once was consumed for exactly one `create_page` call;
- Atlas recorded `dispatch_succeeded` and the MCP returned a non-error result;
- a second exact call required a new confirmation, was denied, raised
  `mcp_confirmation_denied`, and recorded no dispatch outcome.

Thus the MCP allowlist controlled exposure while Atlas separately controlled
authority to dispatch.

## Independent Docmost inspection

The isolated database had zero pages before the write. After the write process
and MCP registration closed, a direct read-only query found exactly one active
page, one exact synthetic-title match, one match in the sole disposable space,
one match attributed to the restricted synthetic identity, and one matching
synthetic body. The count remained one after the denied call, proving that only
the approved mutation reached Docmost.

## Deployment-disabled and cleanup result

- Every isolated Atlas runtime removed its temporary MCP registration.
- Live canonical Atlas contained zero configured `docmost-local-mcp` servers.
- The isolated Compose project ended with zero containers and zero volumes.
- Remote session/config state and disposable `.env` were overwritten and
  removed; no password or reusable credential file had been persisted.
- Local isolated Atlas databases and the temporary exact-source export were
  removed after sanitized evidence capture.
- The distinct production Docmost project retained its three running
  containers and was never targeted.
- No production Atlas configuration, release, merge, or deployment was enabled.

## Acceptance mapping

| Criterion | Result | Evidence |
| --- | --- | --- |
| Digest-verified fork launch without npm downloader | Pass | Exact fork commit/tree, recalculated binary digest, literal absolute-path SSH stdio launch. |
| Atlas read-only run with no write tools | Pass | Exact ten-tool inventory and three non-error bounded reads. |
| Separately allowlisted and Atlas-confirmed write | Pass | Exact `create_page`-only exposure, approve-once dispatch, denial without dispatch, and one independently inspected page. |
| Sanitized identity/config/inventory/confirmation/result/disabled report | Pass | This report, inspected JSON, retained execution record, and final zero-resource assertions. |

This validation authorizes no production enablement. The deployment gate remains
closed pending the workflow's successor independent security re-audit and final
release decision.
