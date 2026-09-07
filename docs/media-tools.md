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
  SHA-256 of its JSON serialization. The hash is informational, not a server CAS token.
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

Existing installations do not silently gain write authority. Atlas's independent
exact-call confirmation remains required, including confirmation of the bytes
being uploaded. Avoid putting base64 or private attachment data in diagnostic logs.

Insertion uses Docmost's JSON `append` operation, never read-modify-replace.
It requires confirmed REST body-update support (v0.70.0+); compatibility evidence
for these specific nodes is v0.95.0. The Markdown `update_page` tool refuses body
replacement if current content cannot be inspected or contains rich media.
Title-only updates remain available. This is an inspection guard, not an atomic
concurrency lock; do not replace bodies while other editors are changing them.

Standalone file deletion, attachment replacement, arbitrary JSON body replacement,
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
