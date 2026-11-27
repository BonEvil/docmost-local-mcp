# Attachments and rich media

The media tools target Docmost Community v0.95.0's authenticated file API and
ProseMirror document format. No enterprise API key is required. They reuse the
existing origin-scoped authentication and never read arbitrary local paths.

## Workflow

1. Read the page ID with `get_page` or `get_page_content`.
2. Call `upload_attachment` with `page_id`, a basename `file_name`,
   `mime_type`, and standard `data_base64`. The decoded size must be 1 byte
   through 2 MiB. The result contains the new attachment ID.
3. Call `insert_media` with that same page ID and attachment ID, `kind`
   (`image`, `video`, or `attachment`), and optional `alt`.
4. Inspect `get_page_content` to verify the rich node. `get_page` also renders
   media links, but Markdown is not a lossless representation.

Upload and insertion are separate confirmed mutations. An upload is not yet a
page-body reference. If insertion fails, retain the returned attachment ID and
inspect before retrying; do not blindly reupload. The connector does not
automatically replay either operation, including on 401 or ambiguous timeouts.

`insert_embed` takes a page UUID and HTTPS `url`. Supply an embed-ready URL,
not necessarily the provider's ordinary watch/share page. The connector does not
fetch that URL or send Docmost credentials to it. Viewing the page may contact the
external provider; embedding depends on its iframe/security policy.

## Read tools

- `get_page_content(slug_id)` returns the complete bounded JSON document and
  SHA-256 of its JSON serialization. The hash supports the structured update
  snapshot guard, not server-side CAS.
- `list_page_attachments(slug_id)` resolves up to 100 distinct referenced IDs,
  including nested nodes. It cannot enumerate uploads not inserted in the page.
- `download_attachment(page_id, attachment_id)` verifies page ownership and
  returns metadata and base64, limited to 2 MiB. It writes no local file.

Only allowlisted metadata is returned: ID, page ID, filename, MIME type, size,
and same-origin relative file URL. Storage paths and creator metadata are omitted.
Redirects are not followed. Both declared response length and streaming bytes are
bounded. MIME types select display nodes; they are not malware/content validation.

## Authorization and preservation

Default mode adds only the three read tools. To enable only media mutations:

```text
--authority-mode=write --write-tools=upload_attachment,insert_media,insert_embed
```

Existing installations do not silently gain write authority. Clients apply their
own authorization policy independently of the connector's write allowlist.
Avoid putting base64 or private attachment data in diagnostic logs.

Insertion uses Docmost's JSON `append` operation, never read-modify-replace.
It requires confirmed REST body-update support (v0.70.0+); compatibility evidence
for these specific nodes is v0.95.0. The Markdown `update_page` tool refuses body
replacement if current content cannot be inspected or contains rich media.
Title-only updates remain available. This is an inspection guard, not an atomic
concurrency lock; do not replace bodies while other editors are changing them.

`update_page` also accepts `content` (a complete ProseMirror document) together
with `expected_content_sha256` from a fresh `get_page_content`. This lets an agent
edit table rows and text or reposition existing images without flattening their
attachment metadata into Markdown. `content` and `markdown` are mutually exclusive.
The connector checks the current snapshot hash, validates document shape and bounds,
and requires every existing rich-media node to remain exactly unchanged, including
attributes and duplicate counts. It refuses new, removed, or altered media nodes.
The `update_page` allowlist entry covers this structured path; no new tool is added.

To put a new image in a table: enable `upload_attachment` and `insert_media` in the
server's write allowlist, upload the bytes, append the returned attachment with
`insert_media`, then read the document again and move that complete image node into
the intended table cell through `update_page(content=..., expected_content_sha256=...)`.
Existing installations must explicitly add these media tools to their arguments
and refresh the MCP catalog; the ordinary page-write allowlist does not expose them.
A data-URI image inside Markdown is not this attachment workflow.

Structured replacement requires confirmed REST body-update support. The snapshot
check is performed before the write and is **not atomic** with it; avoid editing
while a human or another agent changes the page. Replacements are never retried
automatically, even on 401. After any ambiguous failure, read back the page before
issuing another write. Read back successful writes to verify the intended content.

Standalone file deletion, attachment replacement, unrestricted JSON body replacement,
audio-specific widgets, and inline insertion at a cursor are not exposed. Docmost
v0.95.0 has no general standalone file-delete route; safe removal remains a UI
operation. The connector does not claim that removing a reference purges storage.

## Verification

`cargo test --locked --test media_test --test mcp_server_test` checks the media
contract, bounds, ownership, no-retry behavior, and router authority metadata.
`media_live_test` is ignored by default. It requires an empty disposable v0.95.0
instance at literal loopback port 3000 and explicit `DOCMOST_MEDIA_DISPOSABLE=yes`;
it creates a synthetic workspace and must never target production. Teardown must
remove its containers, volumes, and network. The synthetic MP4 fixture verifies the
video node and byte transfer, not video playback. External iframe rendering is
provider-dependent and is not implied by a successful REST insertion.


Operational tool failures now return normal MCP results with `isError: true` and
the existing diagnostic text. Invalid arguments and unknown tools remain JSON-RPC
errors. An ordinary media guard rejection does not end the MCP connection.
