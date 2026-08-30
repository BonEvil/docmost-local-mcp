#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 ARTIFACT_DIR VERSION COMMIT OUTPUT" >&2
  exit 64
fi

artifact_dir=$1
version=$2
commit=$3
output=$4

[[ $version =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]] || {
  echo "version must be a v-prefixed release tag" >&2
  exit 65
}
[[ $commit =~ ^[0-9a-f]{40}$ ]] || {
  echo "commit must be a full lowercase Git commit SHA" >&2
  exit 65
}
command -v jq >/dev/null || { echo "jq is required" >&2; exit 69; }

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else echo "sha256sum or shasum is required" >&2; exit 69
  fi
}

artifacts=()
while IFS= read -r artifact; do
  artifacts+=("$artifact")
done < <(find "$artifact_dir" -maxdepth 1 -type f -name 'docmost-local-mcp-*' -print | LC_ALL=C sort)
[[ ${#artifacts[@]} -gt 0 ]] || { echo "no release binaries found" >&2; exit 66; }

items='[]'
for artifact in "${artifacts[@]}"; do
  name=$(basename "$artifact")
  digest=$(sha256_file "$artifact")
  size=$(wc -c < "$artifact" | tr -d ' ')
  items=$(jq -c \
    --arg name "$name" \
    --arg digest "$digest" \
    --argjson size "$size" \
    '. + [{name: $name, sha256: $digest, size: $size}]' <<<"$items")
done

jq -S -n \
  --arg version "$version" \
  --arg commit "$commit" \
  --arg repository "https://github.com/BonEvil/docmost-local-mcp" \
  --arg rust "1.98.0" \
  --arg cargo "cargo build --locked --release --no-default-features" \
  --arg lock_sha256 "$(sha256_file Cargo.lock)" \
  --arg workflow ".github/workflows/release.yml" \
  --argjson artifacts "$items" \
  '{
    schemaVersion: 1,
    repository: $repository,
    version: $version,
    source: {commit: $commit, workflow: $workflow},
    build: {rust: $rust, command: $cargo, cargoLockSha256: $lock_sha256},
    artifacts: $artifacts
  }' > "$output"
