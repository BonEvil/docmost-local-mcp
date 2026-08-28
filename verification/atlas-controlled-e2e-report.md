# Sanitized Atlas controlled end-to-end validation report

**Status:** blocked closed on 2026-08-28. The current Atlas runtime cannot
independently confirm generic MCP writes, and it rejected the reviewed binary's
non-production stdio launch during Add-MCP validation. No write was attempted,
no Docmost result was created, and deployment remains disabled.

This report is intentionally sanitized. It omits the private endpoint, host
address, credentials, tokens, session material, private identifiers, and
content. The inspected launch shape is retained separately in
[`atlas-e2e-inspected-config.json`](atlas-e2e-inspected-config.json).

## Candidate identity and provenance

| Field | Observed value |
| --- | --- |
| Hardened candidate commit | `254b124ab89c6d5e3623ae99aa30583a2a43d632` |
| Candidate tree | `c481ab19824cd0d13ad2be922ed40ee8e5ae3cdc` |
| Current card base commit | `3909aff52828cf9f193407ffcffaabcc1e54291d` |
| Reviewed Linux x86_64 binary SHA-256 | `78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1` |
| Remote digest recalculated this run | exact match |
| Remote executable present this run | yes |
| Atlas runtime source commit inspected | `746d080ef6a84c3223bf106a2db2a84f1b106ec2` |

The predecessor's manifest records a locked, no-default-features Rust 1.98.0
build from the candidate. This run recalculated the SHA-256 of the exact remote
binary before attempting Atlas configuration. The configured command shape
invoked `/usr/bin/ssh` directly with literal arguments and then the absolute
reviewed fork binary path. It contained no shell wrapper, npm, `npx`, package
downloader, secret, or credential argument.

## Non-production configuration inspection and probe

Atlas's live Add-MCP API was given a default-authority configuration for the
private isolated HTTPS origin. No `--authority-mode` or `--write-tools` override
was present, so the fork's fail-closed default was read-only. Atlas returned:

```text
HTTP 400
code=connection_failed
message=unhandled errors in a TaskGroup (1 sub-exception)
```

Atlas persists an MCP only after the short-lived initialize and tool-list probe
succeeds. A fresh live server listing after the failure contained zero servers
named `docmost-local-mcp`; therefore the rejected configuration was not
persisted or enabled. This run made no second write-authority configuration
because the default process had not passed Atlas validation.

The predecessor's direct MCP compatibility run successfully negotiated
protocol `2025-03-26` with the same binary. That is evidence about the candidate,
not evidence that this Atlas runtime completed its own E2E run.

## Tool inventories

The exact candidate inventories remain useful diagnostic inputs but must not be
misrepresented as Atlas results:

### Candidate default inventory observed by the compatibility predecessor

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

All ten had `readOnlyHint: true`; a direct `create_page` request returned `tool
not found`.

### Candidate explicit-write inventory observed by the compatibility predecessor

The separate predecessor process used the exact allowlist
`create_page,update_page,move_page,create_comment,update_comment`. Its inventory
was the ten reads plus exactly those five writes, and an unallowlisted
`create_space` request returned `tool not found`.

### Atlas inventory in this run

No Docmost inventory was registered because Atlas's mandatory Add-MCP probe
failed. Consequently bounded reads did not run through Atlas, and no
write-authority inventory was exposed through Atlas.

## Independent Atlas-confirmation inspection

The current Atlas source was clean at commit
`746d080ef6a84c3223bf106a2db2a84f1b106ec2`. These exact files were inspected:

| File | SHA-256 | Relevant observed behavior |
| --- | --- | --- |
| `backend/mcp_store.py` | `018bbd7638b42ef5c4cbb0af5725bacbe054eccc17761916b8abb5fb514e101e` | Lines 677-710 map MCP annotations into descriptive `effect` and idempotency metadata. |
| `backend/app.py` | `6cd853f9c1e96dc4090fc9ba551ca06ef53bd68889fd535d4436ddc2c804cff9` | Lines 523-542 check lifecycle, effect filtering, availability, and schema, then dispatch a generic MCP call directly. No confirmation authority is checked. |
| `backend/interfaces/codex.py` | `0155887aaa0b3d875d5dc0c1938ecfb2af793f2744b7d7be9f34bfc84dacd06f` | Lines 376-404 set Codex `approvalPolicy` to `never` for both thread start and resume. |
| `docs/build-journal.md` | `27ae093a6e9688de3aff1576ce24ff5739f1d0a32c01ba162cef7e97aae85ae6` | Lines 89-93 explicitly state that Atlas does not yet expose an approval-request interface; existing confirmation modals apply to destructive Atlas application actions. |

The source inspection demonstrates that a generic MCP mutation, once loaded and
available, reaches `mcp_manager.call` without an independent per-call Atlas
confirmation. The effect metadata does not create authority. Presenting a
conversational User Choice or relying on MCP `destructiveHint` would simulate,
not demonstrate, the required enforcement and was therefore not attempted.

## Disposable target and Docmost inspection

The approved remote host remained reachable, and the reviewed binary remained
present and digest-matched. The compatibility Compose project had exactly:

```text
containers=0
volumes=0
```

That state is consistent with the predecessor's recorded cleanup. Starting a
fresh stack would not cure Atlas's missing confirmation boundary and would
require a new restricted authentication bootstrap before any bounded read.
No stack was started, no authentication material was read or copied, no page or
comment was created, and there was no Docmost result to claim as inspected in
this run.

## Deployment-disabled evidence

- Live Atlas contained zero configured `docmost-local-mcp` servers after the
  failed validation probe.
- The rejected non-production configuration was never persisted or enabled.
- The compatibility project remained stopped with zero containers and volumes.
- `config/atlas-mcp.production.example.json` remains an example only and points
  to the absolute installed fork path in default read-only authority.
- No production configuration, service, content, repository release, or
  deployment state was changed.

## Exact blocker and required remediation

Two conditions must be resolved outside this card before the acceptance test can
be rerun:

1. Atlas must provide and enforce an independent confirmation boundary for
   generic MCP tools classified as write or destructive. The enforcement must
   occur before MCP dispatch and must not trust MCP annotations as authorization.
2. Atlas's local stdio probe must successfully initialize and enumerate this
   reviewed `rmcp` server through the approved direct launch path, with an error
   that preserves the underlying non-secret protocol failure if negotiation
   fails.

After those product/runtime changes are independently reviewed, recreate the
isolated Docmost v0.95.0 stack and restricted session, configure the default
read-only process, perform bounded reads, separately configure the smallest
write allowlist, prove confirmation denial and approval paths, perform the
authorized disposable write, independently inspect the result, restore
read-only state, and leave deployment disabled.
