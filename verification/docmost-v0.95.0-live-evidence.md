# Sanitized live compatibility execution record

Captured during the 2026-08-28 compatibility run. This is the bounded
execution evidence supporting `docmost-v0.95.0-compatibility-report.md`.
It intentionally omits the private endpoint, token/session values, page IDs,
user IDs, workspace IDs, and page/comment bodies.

## Candidate and server checks

Remote command (over the approved SSH boundary):

```text
shasum -a 256 /home/danielperson/services/docmost-atlas-compat/bin/docmost-local-mcp
```

Observed output:

```text
78133af4492f2333f63f3ff8e673a16f8713e5fffcaaf2e0c0ba4255c5a155b1  docmost-local-mcp
```

The remote container inventory, filtered to the compatibility Compose project,
contained exactly these three images before cleanup:

```text
docmost/docmost:0.95.0
postgres:16-alpine
redis:7.2-alpine
```

The origin-bound local session-state directory and its two state files had
observed modes `0700`, `0600`, and `0600`, respectively. No state filename or
content was retained here.

## Read-only process evidence

The exact verified binary was launched without authority environment overrides.
MCP `initialize` returned protocol version `2025-03-26`; `tools/list` returned
exactly the following ten names, all with `readOnlyHint: true`:

```text
list_workspace_members
get_current_user
search_docs
list_pages
get_comments
list_child_pages
search_pages
list_spaces
get_space
get_page
```

A direct `tools/call` for `create_page` returned the MCP error:

```text
code=-32602 message=tool not found
```

Bounded initial reads returned: one current restricted identity, one workspace
member, one disposable space, zero initial pages in that space, and zero search
results for the synthetic compatibility probe. `get_space` identified the
current identity role as `admin` in the disposable space only.

## Allowlisted write-process evidence

A separately launched binary process used only:

```text
DOCMOST_AUTHORITY_MODE=write
DOCMOST_WRITE_TOOLS=create_page,update_page,move_page,create_comment,update_comment
```

Its `tools/list` result contained the ten read tools and exactly these five
write names:

```text
create_page
update_page
move_page
create_comment
update_comment
```

An unallowlisted `create_space` call returned:

```text
code=-32602 message=tool not found
```

The following tool calls all returned successful MCP results, using only IDs
returned by the isolated instance: two `create_page` calls (synthetic parent
and child), one `update_page`, one `move_page` placing the child under the
parent, one `create_comment`, and one `update_comment`.

## Independent result inspection

A new default read-only binary process, not the write process, confirmed the
updated synthetic child via `get_page`, one child under the synthetic parent via
`list_child_pages`, and one comment via `get_comments`.

Separately, a direct SQL query against only the isolated compatibility database
returned these sanitized counts before cleanup:

```text
synthetic_pages=2
nested_child=1
synthetic_comments=1
```

## Bounded production and cleanup evidence

Before cleanup, Docker label filters showed exactly three compatibility-project
containers and three compatibility-project volumes. A distinct production
Compose project showed three containers. The cleanup command named only the
three compatibility containers and three compatibility volumes that had just
been enumerated; it did not name a production resource.

The post-cleanup assertion output was:

```text
cleanup=pass production_project_containers=3
```

The assertion checked zero remaining resources with the compatibility project
label and exactly three remaining containers with the distinct production
project label. No ordinary production Docmost page, space, identity, database,
endpoint, or container was accessed by this execution record.
