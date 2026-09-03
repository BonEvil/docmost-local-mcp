#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
expected=0.9.5
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

cargo_version=$(sed -n '/^\[package\]$/,/^\[/s/^version = "\([^"]*\)"$/\1/p' "$repo_root/Cargo.toml")
lock_version=$(awk '
  $0 == "name = \"docmost-local-mcp\"" { package = 1; next }
  package && /^version = / { gsub(/^version = \"|\"$/, ""); print; exit }
' "$repo_root/Cargo.lock")
npm_version=$(jq -er '.version' "$repo_root/npm/launcher/package.json")
[[ $cargo_version == "$expected" ]] || { echo "Cargo release identity is $cargo_version, expected $expected" >&2; exit 1; }
[[ $lock_version == "$expected" ]] || { echo "Cargo.lock release identity is $lock_version, expected $expected" >&2; exit 1; }
[[ $npm_version == "$expected" ]] || { echo "npm release identity is $npm_version, expected $expected" >&2; exit 1; }
grep -q 'prepared source release identity is \*\*v0.9.5\*\*' "$repo_root/docs/atlas-release-integrity.md"

mkdir -p "$test_root/artifacts"
printf 'synthetic release binary\n' > "$test_root/artifacts/docmost-local-mcp-linux-x64"
(
  cd "$repo_root"
  bash scripts/generate-release-manifest.sh \
    "$test_root/artifacts" "v$expected" 0123456789abcdef0123456789abcdef01234567 "$test_root/manifest.json"
)
jq -e --arg version "v$expected" '.version == $version' "$test_root/manifest.json" >/dev/null
jq -e --arg version "$expected" '
  .product == {name:"docmost-local-mcp",title:"Docmost MCP",version:$version}
' "$test_root/manifest.json" >/dev/null

if (
  cd "$repo_root"
  bash scripts/generate-release-manifest.sh \
    "$test_root/artifacts" "v0.9.4" 0123456789abcdef0123456789abcdef01234567 "$test_root/stale.json"
) >/dev/null 2>&1; then
  echo "manifest generator accepted a stale release tag" >&2
  exit 1
fi

grep -q 'test "\$GITHUB_REF_NAME" = "v\$product_version"' "$repo_root/.github/workflows/release.yml"
grep -q 'test "\$binary_identity" = "docmost-local-mcp \$product_version"' "$repo_root/.github/workflows/release.yml"

# Cargo.lock dependency versions are not project release metadata. The package-version
# assertion above checks the docmost-local-mcp lockfile entry directly.
if git -C "$repo_root" grep -n -E '0\.9\.4|v0\.9\.4' -- \
  Cargo.toml npm/launcher/package.json .github scripts tests \
  ':!.github/workflows/v0.9.4-platform-evidence.yml' \
  ':!.github/workflows/v0.9.4-retain-platform-evidence.yml' \
  ':!tests/fixtures/cosign-v3.0.3-linux-amd64-assets.txt' \
  ':!tests/cosign_installer_contract_test.sh' \
  ':!tests/version_consistency_test.sh' >/dev/null; then
  echo "active release metadata still contains the superseded project version" >&2
  exit 1
fi

echo "v0.9.5 source, package, CLI, MCP, manifest, workflow, test, and documentation identity: ok"
