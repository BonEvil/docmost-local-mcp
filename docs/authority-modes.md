# MCP authority modes

`docmost-local-mcp` starts read-only. Its default router contains exactly:

`list_spaces`, `search_docs`, `search_pages`, `get_space`, `get_page`,
`list_pages`, `list_child_pages`, `get_comments`, `list_workspace_members`, and
`get_current_user`.

No persistent mutation is registered in this mode.

## Enabling a bounded write surface

Write exposure requires two independent startup values:

```text
--authority-mode=write
--write-tools=create_page,update_page
```

The environment equivalents are `DOCMOST_AUTHORITY_MODE=write` and
`DOCMOST_WRITE_TOOLS=create_page,update_page`. CLI values take precedence over
environment values.

The allowlist is exact and comma-separated. Its only valid names are:

`create_page`, `update_page`, `duplicate_page`, `copy_page_to_space`,
`move_page`, `move_page_to_space`, `create_space`, `update_space`,
`create_comment`, `update_comment`, `delete_page`, `delete_space`, and
`delete_comment`.

Write mode with an empty allowlist, an allowlist in read-only mode, unknown or
read-tool names, duplicates, and empty entries are startup errors. The server
does not broaden the allowlist: for example, allowing `create_page` exposes that
one write alongside the ten read tools, not every write.

## Mutation annotations

Annotations describe effects; they do not authorize them.

| Tool | Read-only | Destructive | Idempotent |
| --- | --- | --- | --- |
| `create_page` | false | false | false |
| `update_page` | false | true | false |
| `duplicate_page` | false | false | false |
| `copy_page_to_space` | false | false | false |
| `move_page` | false | true | false |
| `move_page_to_space` | false | true | false |
| `create_space` | false | false | false |
| `update_space` | false | true | false |
| `create_comment` | false | false | false |
| `update_comment` | false | true | false |
| `delete_page` | false | true | false |
| `delete_space` | false | true | false |
| `delete_comment` | false | true | false |

Creation and copy operations add data without overwriting existing data, so
their destructive hint is false. Move and update operations change or replace
existing persistent state, so their destructive hint is true. Idempotency is
conservatively false for every mutation: retries can duplicate created data or
can alter timestamps, ordering, and other server-maintained state.
Deletes are also conservatively non-idempotent. They never automatically retry,
because an interrupted response can leave the remote outcome ambiguous.

## Atlas control remains mandatory

MCP annotations are advisory metadata, not an authorization boundary. Atlas
must independently keep its confirmation controls enabled for every exposed
write tool. Enabling a tool in this server only makes it callable; it does not
authorize Atlas to execute the mutation without confirmation. Use the narrowest
allowlist needed for the session and return to the default read-only launch when
the write task is complete.
