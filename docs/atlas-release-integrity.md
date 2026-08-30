# Atlas release integrity

Atlas production must execute a reviewed binary from this fork. The supported
path is the absolute executable in `config/atlas-mcp.production.example.json`;
the npm launcher and upstream downloader are not part of production deployment.

## Pinned release inputs and outputs

The release workflow checks out an exact commit, uses Rust 1.98.0 and
`Cargo.lock`, disables incremental compilation, remaps the checkout path, and
sets `SOURCE_DATE_EPOCH` to the reviewed commit time. Release profile settings
fix codegen units, LTO, and symbol stripping. GitHub Actions are full-SHA pinned.
The Dockerfile pins the multi-platform Rust and Debian OCI indexes by digest.

Each tag build produces versioned platform binaries, `SHA256SUMS`, and a
canonical `release-manifest.json`. The manifest records the full source commit,
tag, build command, Rust version, lockfile digest, artifact names, sizes, and
SHA-256 digests. The release workflow keyless-signs that manifest with cosign
and publishes its Sigstore bundle. It also requests GitHub artifact attestations
for the binaries. These are complementary: the Atlas installer makes the signed
manifest and reviewed digest mandatory; GitHub attestations provide an
additional provider-verifiable provenance record.

## Review, install, and launch

1. Review the exact release commit and record the tag, full 40-character commit
   SHA, and platform-binary SHA-256 from `release-manifest.json`. Do not infer
   these values from a moving branch or from a filename.
2. Install cosign v3.0.3 from its official release using its published signature
   verification procedure. Ensure `curl`, `jq`, and `sha256sum` (or `shasum`) are
   available.
3. Create the destination directory on the same filesystem as its final path,
   owned by the Atlas service account and not writable by untrusted users.
4. Run the installer with the reviewed identities:

   ```bash
   scripts/install-atlas.sh \
     --version vX.Y.Z \
     --expected-commit FULL_40_CHARACTER_REVIEWED_COMMIT \
     --expected-sha256 REVIEWED_64_CHARACTER_BINARY_SHA256 \
     --install-path /opt/atlas/mcp/docmost-local-mcp
   ```

The Linux/macOS installer follows at most five redirects itself and permits only HTTPS
responses from `github.com`, `objects.githubusercontent.com`, and
`release-assets.githubusercontent.com`. It caps binaries at 128 MiB and metadata
at 1 MiB. A unique staging directory is created beside the destination. The
installer rejects network/partial writes, size overruns, unapproved redirects,
invalid Sigstore provenance identity, tag/commit/digest disagreement, duplicate
manifest entries, and artifact digest mismatch. Only after every check succeeds
does it set mode 0755 and atomically rename the staged binary over the final
path. Existing executables are never treated as evidence and remain unchanged
on a failed attempt.

Inspect the production configuration and launch target before restart:

```bash
jq -er '.mcpServers.docmost.command' config/atlas-mcp.production.example.json
test "$(jq -r '.mcpServers.docmost.command' config/atlas-mcp.production.example.json)" = \
  /opt/atlas/mcp/docmost-local-mcp
test -x /opt/atlas/mcp/docmost-local-mcp
```

The example starts read-only. Any write authority remains governed by the
separate authority-mode operating policy.

## Provider-side protections required by the release gate

The repository workflow cannot create its own independent provider boundary.
Before the first production release, an administrator must configure and retain
read-back evidence for all of these controls:

- protect `main` against force updates/deletion and require reviewed pull requests
  plus the exact `Integrated security and release gate` check;
- protect `v*` release tags against update/deletion and permit their creation only
  through the reviewed signed-tag procedure;
- restrict Actions to approved full-SHA-pinned actions and prevent workflow
  bypass by untrusted actors;
- enable immutable releases or an equivalent no-replacement policy;
- retain Sigstore transparency-log access and GitHub artifact attestations; and
- configure the `protected-release` environment with required reviewer approval
  and narrowly scoped publication authority.

The release workflow binds its privileged job to that exact environment and
rejects lightweight/unsigned tags, commits outside `origin/main`, and missing or
non-successful exact-commit terminal gates before building. Provider settings
remain externally verified facts, not claims inferred from repository YAML.
Publishing a release and enabling deployment remain separately unauthorized.

## Immutable Debian inputs

The container build replaces moving Debian repositories with the dated snapshot
declared in `config/debian-snapshot.sources` and pins every directly requested apt
package to an exact version. Apt still verifies Debian Release signatures and
package hashes. The minimal runtime base initially lacks CA roots, so its snapshot
bootstrap uses HTTP under that signed/hash-bound apt trust model; the pinned
`ca-certificates` package is installed before the MCP binary runs.
