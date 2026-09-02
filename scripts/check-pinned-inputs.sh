#!/usr/bin/env bash
set -euo pipefail

fail() { echo "check-pinned-inputs: $*" >&2; exit 1; }

while IFS= read -r use; do
  [[ $use =~ uses:[[:space:]]+[^[:space:]@]+@[0-9a-f]{40}([[:space:]]|$) ]] || \
    fail "GitHub Action is not pinned to a full commit: $use"
done < <(grep -R -h -E '^[[:space:]]*uses:' .github/workflows)

grep -q '^channel = "1.98.0"$' rust-toolchain.toml || fail "Rust toolchain is not pinned"
grep -q '^FROM .*@sha256:[0-9a-f]\{64\}' Dockerfile || fail "container inputs are not digest-pinned"
[[ $(grep -c '^FROM .*@sha256:[0-9a-f]\{64\}' Dockerfile) -eq 2 ]] || fail "every container stage must be digest-pinned"
grep -q 'cargo build --locked --release --no-default-features' .github/workflows/release.yml || fail "release build is not locked"
grep -q 'sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6 # v4.1.2' .github/workflows/release.yml || \
  fail "cosign installer is not pinned to the reviewed v4 source identity"
grep -q 'cosign-release: v3.0.3' .github/workflows/release.yml || fail "cosign release is not pinned"
! grep -q 'sigstore/cosign-installer@d7543c93d881b35a8faa02e8e3605f69b7a1ce62' .github/workflows/release.yml || \
  fail "cosign-installer v3 cannot install cosign v3 bundle releases"
grep -q 'cosign sign-blob --bundle' .github/workflows/release.yml || fail "release manifest is not signed"
grep -q 'actions/attest-build-provenance@[0-9a-f]\{40\}' .github/workflows/release.yml || fail "build provenance is absent or unpinned"
grep -q 'snapshot.debian.org/archive/debian/[0-9]\{8\}T[0-9]\{6\}Z' config/debian-snapshot.sources || fail "Debian package repository is not snapshot-pinned"
grep -q 'snapshot.debian.org/archive/debian-security/[0-9]\{8\}T[0-9]\{6\}Z' config/debian-snapshot.sources || fail "Debian security repository is not snapshot-pinned"
[[ $(grep -c '^Check-Valid-Until: no$' config/debian-snapshot.sources) -eq 2 ]] || fail "snapshot validity policy is incomplete"
if grep -E '^[[:space:]]+[A-Za-z0-9.+-]+[[:space:]]*([\\])?$' Dockerfile | grep -v '=' >/dev/null; then
  fail "Dockerfile contains an unversioned apt package"
fi
grep -q 'environment: protected-release' .github/workflows/release.yml || fail "release job is not bound to the protected environment"
grep -q 'verify-release-context.sh' .github/workflows/release.yml || fail "release context verification is absent"
! grep -R -n -E '"command"[[:space:]]*:[[:space:]]*"npx"|npx -y' config docs/atlas-release-integrity.md || \
  fail "production Atlas path references npx"

echo "immutable release inputs: ok"
