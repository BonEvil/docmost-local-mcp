#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
expected=0.9.4
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
grep -q 'prepared source release identity is \*\*v0.9.4\*\*' "$repo_root/docs/atlas-release-integrity.md"

mkdir -p "$test_root/artifacts"
printf 'synthetic release binary\n' > "$test_root/artifacts/docmost-local-mcp-linux-x64"
(
  cd "$repo_root"
  bash scripts/generate-release-manifest.sh \
    "$test_root/artifacts" "v$expected" 0123456789abcdef0123456789abcdef01234567 "$test_root/manifest.json"
)
jq -e --arg version "v$expected" '.version == $version' "$test_root/manifest.json" >/dev/null

if git -C "$repo_root" grep -n -E '0\.9\.3|v0\.9\.3' -- \
  Cargo.toml Cargo.lock npm/launcher/package.json .github scripts tests \
  ':!tests/fixtures/cosign-v3.0.3-linux-amd64-assets.txt' \
  ':!tests/cosign_installer_contract_test.sh' >/dev/null; then
  echo "active release metadata still contains the superseded project version" >&2
  exit 1
fi

echo "v0.9.4 source, package, manifest, workflow, test, and documentation identity: ok"
