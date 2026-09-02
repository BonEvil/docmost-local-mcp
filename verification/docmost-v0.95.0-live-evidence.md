# Sanitized exact-candidate live compatibility execution record

> Superseded for v0.9.4 by
> [`v0.9.4-refresh-evidence-r2.md`](v0.9.4-refresh-evidence-r2.md). The record
> below remains the historical v0.9.3 execution.

Captured on 2026-08-31 for the v0.9.3 candidate. This is the retained observed
output supporting `docmost-v0.95.0-compatibility-report.md`. Commands that
handled secrets or identifiers are described but not reproduced. No endpoint,
credential, token, cookie, session filename, identifier, content canary, or
sensitive response value is retained.

## Candidate and build observations

```text
integration_commit=b93024c6094abe5044b56f32991e39d650c6c9e5
inherited_candidate_commit=a33895bef1c66c3bb0855c4d2ee06cef252c020f
integration_tree=327d122cf9695b586e6999142a101b86bda4f67a
candidate_tree=327d122cf9695b586e6999142a101b86bda4f67a
cargo_lock_sha256=3f2a639c0bed73088017f70fe9564a7d1696c24b4884e51bccf20284ce4754b9
build_host=Linux x86_64
build_image_digest=sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157
build_command=cargo build --locked --release --no-default-features
binary_format=ELF 64-bit x86-64 PIE stripped
binary_sha256=7585777a8423f0b4c867331c31f250cf3aa7b0fef4f38a41c88e89287f66cd52
```

The source was mounted read-only in the digest-pinned build container. The
binary digest shown above was recalculated immediately before cleanup.

## Disposable boundary and baseline observations

Before creation:

```text
ordinary_project_containers=3
ordinary_inventory_sha256=17c351c06381f653bd2332f1db47eaeefdf7c480819510ff394c924689408c80
ordinary_started_sha256=7702be144f44fbcc5e040570cf2906f619e97e281382c9001b31e5434be6e007
serve_config_sha256=45a54a9f3b304d934f54bb66853ebcebae88337677d65b6dfadc073c7aa11401
ordinary_3001_listener=present
disposable_3301_listener=absent
compatibility_containers=0
compatibility_volumes=0
```

After the isolated project started:

```text
server_package_version=0.95.0
docmost_image_digest=sha256:41c8d777cf23c74e78f94e676aec328b7d7856f48df5e573543dac68d371e37c
compatibility_containers=3
compatibility_volumes=3
images=docmost/docmost:0.95.0,postgres:16-alpine,redis:7.2-alpine
```

The existing private HTTPS route and the ordinary root route were not changed.
No ordinary Docmost endpoint or content was read.

## MCP and compatibility observations

The sanitized harness output was:

```text
server_version=0.95.0
protocol_version=2025-03-26
default_read_tools=10 default_write_tools=0 all_write_rejections=10
allowlisted_write_tools=5 unallowlisted_write_rejections=5
read_calls_exercised=10 write_calls_exercised=6
fresh_readback=pass expiry_requires_interactive=pass session_relogin=pass
state_modes=0700/0600 credential_files=0 origin_forget=pass
```

The exact default read inventory was:

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

Every mutation name returned this error shape in the default process:

```text
code=-32602 message=tool not found
```

The separate write process exposed the ten reads plus exactly:

```text
create_comment
create_page
move_page
update_comment
update_page
```

The other five mutation names returned the same `tool not found` result. The
allowlisted process completed two page creations, one page update, one move,
one comment creation, and one comment update. A new default process then
completed `get_page`, `list_child_pages`, and `get_comments` and observed the
expected synthetic state.

Direct read-only inspection of only the isolated database returned:

```text
synthetic_pages=2
nested_child=1
updated_page_body=1
synthetic_comments=1
updated_comment_body=1
```

## Hostile diagnostic and input-bound observations

After the first peer review identified the missing retained diagnostic cases,
the exact `b93024c` source archive was rebuilt in the same pinned Rust image.
The rebuilt binary had the identical SHA-256
`7585777a8423f0b4c867331c31f250cf3aa7b0fef4f38a41c88e89287f66cd52`.

Each case used a dedicated synthetic loopback origin and a disposable
session-only home. Hostile bodies contained a synthetic canary. The harness
checked candidate-visible errors for the canary, origin, URL, and response
excerpt, and re-enumerated the ten-tool inventory after every failure to prove
the process remained usable.

Sanitized observed output:

```text
redirect=pass target_hits=0 elapsed_seconds=0.0
timeout=pass elapsed_seconds=30.0
declared_oversize_response=pass elapsed_seconds=0.0
chunked_oversize_response=pass elapsed_seconds=0.0
permission_denied=pass status=403 body_omitted_bytes=718 elapsed_seconds=0.0
server_error=pass status=500 body_omitted_bytes=718 elapsed_seconds=0.0
oversized_input=pass network_hits=0
diagnostic_redaction=pass canary_origin_url_leaks=0 process_reusable_after_each_failure=pass
```

The redirect target received no request. Timeout occurred at the code-owned
30-second production deadline. Both response-body forms failed at the
8,388,608-byte production cap. The one-byte-oversized search input was rejected
before the hostile server received a request. Both hostile 718-byte error
bodies were omitted for `403` and `500` responses.

## Cleanup and invariance observations

The disposable Compose project was stopped with its exact project name and
compose file, including volumes and orphans. The root-owned Cargo output was
ownership-normalized only inside the resolved disposable directory before that
directory was deleted. Local card-created build and harness artifacts were also
removed.

Final assertions:

```text
cleanup=pass
compat_containers=0
compat_volumes=0
compat_networks=0
remote_home_binary_credentials_harness=absent
local_build_home_binary_credentials_harness=absent
local_build_container=absent
negative_cleanup=pass
negative_remote_source_target_cargo_home_binary_harness=absent
negative_build_cleanup_containers=0
negative_temporary_homes=0
ordinary_inventory_sha256=17c351c06381f653bd2332f1db47eaeefdf7c480819510ff394c924689408c80
ordinary_started_sha256=7702be144f44fbcc5e040570cf2906f619e97e281382c9001b31e5434be6e007
serve_config_sha256=45a54a9f3b304d934f54bb66853ebcebae88337677d65b6dfadc073c7aa11401
ordinary_3001_listener=present
disposable_3301_listener=absent
```

The three hashes exactly match the pre-test baseline. The ordinary containers
were neither recreated nor restarted, the ordinary listener remained present,
and the complete Serve configuration—including the stale `:8443` route—was
unchanged.
