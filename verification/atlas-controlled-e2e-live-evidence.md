# Sanitized Atlas controlled E2E execution record

Captured on 2026-08-30. This record omits private endpoints, host addresses,
credentials, tokens, cookies, private identifiers, and all non-synthetic
content. It records only exact public candidate identities, tool names,
synthetic test labels, aggregate counts, and authorization outcomes.

## Exact identities

```text
fork_commit=254b124ab89c6d5e3623ae99aa30583a2a43d632
fork_tree=c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc
binary_sha256=78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1
atlas_commit=efad3719b67fc9949be3809a7d07b297a64de10d
atlas_tree=58d8b8c5d330c905ef70b5be33b06883c0a57ae6
```

The Atlas source tree was exported read-only from the exact commit. Inspection
found the reviewed legacy stdio worker configuration. The remote binary digest
was recalculated both before execution and after cleanup and matched exactly.
The direct literal-argument launch used `/usr/bin/ssh`, `/usr/bin/env` to set a
disposable `HOME`, and the absolute reviewed fork binary. No shell, npm, `npx`,
downloader, or credential argument was used.

## Disposable target baseline

The isolated Compose project advertised exactly these images:

```text
docmost/docmost:0.95.0
postgres:16-alpine
redis:7.2-alpine
```

The Docmost package reported `0.95.0`. The sanitized initial database counts
were:

```text
users=1
workspaces=1
spaces=1
pages=0
```

The synthetic identity password was rotated only inside the disposable
database. Login produced session-only origin-bound state with directory mode
`0700` and two files at mode `0600`; no password or reusable credential file
was persisted.

## Default read-only Atlas runs

Each operation used a separately created isolated Atlas runtime and a default
fork process with no authority override. Every Add-MCP probe negotiated
protocol `2025-03-26` and returned exactly this inventory:

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

Every inventory row had `readOnly=true`; no write-capable name was present.
Three bounded calls completed through the real Atlas generic MCP path:

```json
{"authorizationStatus":"consumed","confirmationEffect":"write","dispatchOutcomeCode":"dispatch_succeeded","mode":"read","preDecisionDispatchBlocked":true,"protocolVersion":"2025-03-26","resultIsError":false,"tool":"get_current_user"}
{"authorizationStatus":"consumed","confirmationEffect":"write","dispatchOutcomeCode":"dispatch_succeeded","mode":"list-spaces","preDecisionDispatchBlocked":true,"protocolVersion":"2025-03-26","resultIsError":false,"tool":"list_spaces"}
{"authorizationStatus":"consumed","confirmationEffect":"write","dispatchOutcomeCode":"dispatch_succeeded","mode":"list-pages","preDecisionDispatchBlocked":true,"protocolVersion":"2025-03-26","resultIsError":false,"tool":"list_pages"}
```

`confirmationEffect=write` is Atlas's fail-closed classification of an
untrusted generic MCP call, not a write tool in the fork inventory. For each
call, the future remained incomplete before the Atlas decision, the projected
confirmation contained no supplied argument value, approve-once was consumed,
the MCP result was non-error, and Atlas recorded `dispatch_succeeded`.

## Explicit allowlisted write run

The separately launched write process added only:

```text
--authority-mode=write
--write-tools=create_page
```

Its inventory was the same ten reads plus exactly `create_page`. It contained
11 tools; `create_space` and every other mutation were absent. Before the
approved call, both the total active-page count and the exact synthetic-title
count were zero.

The sanitized Atlas result was:

```json
{"authorizationStatus":"consumed","confirmationEffect":"write","deniedAuthorizationStatus":"denied","deniedDispatchOutcomeCode":null,"dispatchOutcomeCode":"dispatch_succeeded","inventory":["create_page","get_comments","get_current_user","get_page","get_space","list_child_pages","list_pages","list_spaces","list_workspace_members","search_docs","search_pages"],"mode":"write","preDecisionDispatchBlocked":true,"protocolVersion":"2025-03-26","resultIsError":false,"tool":"create_page"}
```

Atlas held the first call before dispatch, projected no space ID, synthetic
title, or synthetic body, consumed approve-once, and recorded
`dispatch_succeeded`. A second exact write entered a new pending confirmation,
remained undispatched, was explicitly denied, returned
`mcp_confirmation_denied`, and retained a null dispatch outcome.

## Independent Docmost result inspection

A direct read-only SQL query against only the isolated Docmost database, after
the Atlas process and temporary MCP registration had closed, returned:

```text
postwrite_pages=1
postwrite_matching_title=1
postwrite_in_disposable_space=1
postwrite_creator_is_restricted_identity=1
postwrite_matching_body=1
```

The single intended synthetic page therefore existed in the sole disposable
space, was created by the restricted synthetic identity, and contained the
intended synthetic body. The exact-title count remaining one also independently
confirms that the denied second call did not create a duplicate.

## Atlas registration and authorization lifecycle

Before isolated runtime cleanup, the four successful runtime databases showed:

```text
get_current_user: registered_servers=0; consumed/dispatch_succeeded=1
list_spaces: registered_servers=0; consumed/dispatch_succeeded=1
list_pages: registered_servers=0; consumed/dispatch_succeeded=1
create_page: registered_servers=0; consumed/dispatch_succeeded=1; denied/no_dispatch=1
```

Every harness removed its temporary MCP registration. The live canonical Atlas
server list contained zero `docmost-local-mcp` entries after the run.

## Corrected transient and cleanup evidence

After the first successful read, the previously supplied session worktree was
mechanically advanced from the reviewed commit back to the old main commit.
Two subsequent Add-MCP attempts failed closed before persistence or dispatch.
The correction exported the exact reviewed commit/tree from Git, verified its
tree identity and legacy-worker setting, and reran only the failed read phases.

Cleanup then removed the exact disposable Compose project and volumes, securely
overwrote and removed its session/config state and `.env`, and removed the local
isolated Atlas runtime databases and temporary source export. Final assertions:

```text
compat_containers=0
compat_volumes=0
runtime_state_files=0
live_canonical_docmost_servers=0
production_containers=3
binary_sha256=78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1
```

No production Docmost endpoint, identity, database, page, space, or content was
read or modified. No production Atlas configuration, release, or deployment was
enabled.
