#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
assets="$repo_root/tests/fixtures/cosign-v3.0.3-linux-amd64-assets.txt"
workflow="$repo_root/.github/workflows/release.yml"
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM
mkdir -p "$test_root/release" "$test_root/download"

while IFS= read -r asset; do
  [[ -n $asset && $asset != \#* ]] || continue
  : > "$test_root/release/$asset"
done < "$assets"

# Regression: cosign-installer v3.10.0 always requested the legacy detached
# signature. The v3.0.3 release did not publish it, so the download failed.
legacy_signature=cosign-linux-amd64.sig
if cp "$test_root/release/$legacy_signature" "$test_root/download/" 2>/dev/null; then
  echo "legacy v0.9.3 signature download unexpectedly succeeded" >&2
  exit 1
fi
[[ ! -e $test_root/download/$legacy_signature ]]

# Sigstore's v4 installer contract for cosign v3 consumes the published bundle.
bundle=cosign-linux-amd64.sigstore.json
cp "$test_root/release/$bundle" "$test_root/download/$bundle"
[[ -f $test_root/download/$bundle ]]

grep -q 'sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6 # v4.1.2' "$workflow"
grep -q 'cosign-release: v3.0.3' "$workflow"
! grep -q 'sigstore/cosign-installer@d7543c93d881b35a8faa02e8e3605f69b7a1ce62' "$workflow"

echo "cosign v3 bundle installer contract and v0.9.3 missing-signature regression: ok"
