#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM
mkdir -p "$test_root/mock-bin" "$test_root/fixtures" "$test_root/install"

commit=0123456789abcdef0123456789abcdef01234567
version=v1.2.3
asset=docmost-local-mcp-linux-x64
printf '#!/bin/sh\necho verified\n' > "$test_root/fixtures/$asset"
digest=$(sha256sum "$test_root/fixtures/$asset" | awk '{print $1}')
jq -S -n \
  --arg version "$version" --arg commit "$commit" --arg name "$asset" --arg digest "$digest" \
  '{schemaVersion:1,version:$version,
    product:{name:"docmost-local-mcp",title:"Docmost MCP",version:"1.2.3"},
    source:{commit:$commit},artifacts:[{name:$name,sha256:$digest,size:24}]}' \
  > "$test_root/fixtures/release-manifest.json"
printf '{"mediaType":"application/vnd.dev.sigstore.bundle.v0.3+json"}\n' \
  > "$test_root/fixtures/release-manifest.sigstore.json"

cat > "$test_root/mock-bin/curl" <<'MOCK_CURL'
#!/usr/bin/env bash
set -euo pipefail
headers="" output="" url=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dump-header|--output|--max-filesize|--connect-timeout|--max-time|--max-redirs|--proto)
      case "$1" in --dump-header) headers=$2 ;; --output) output=$2 ;; esac
      shift 2 ;;
    --tlsv1.2|--silent|--show-error) shift ;;
    *) url=$1; shift ;;
  esac
done
name=${url##*/}
if [[ $url == https://github.com/* ]]; then
  if [[ ${TEST_SCENARIO:-success} == unapproved && $name == docmost-local-mcp-* ]] ||
     [[ ${TEST_SCENARIO:-success} == redirected-verification && $name == release-manifest.sigstore.json ]]; then
    printf 'HTTP/1.1 302 Found\r\nLocation: https://evil.example/%s\r\n\r\n' "$name" > "$headers"
  else
    printf 'HTTP/1.1 302 Found\r\nLocation: https://release-assets.githubusercontent.com/%s\r\n\r\n' "$name" > "$headers"
  fi
  : > "$output"
  exit 0
fi
printf 'HTTP/1.1 200 OK\r\n\r\n' > "$headers"
if [[ $name == release-manifest.sigstore.json ]]; then
  case ${TEST_SCENARIO:-success} in
    missing-verification) rm -f "$output"; exit 22 ;;
    malformed-verification) printf '{not-json\n' > "$output"; exit 0 ;;
    mismatched-verification) printf '{"mediaType":"application/vnd.dev.sigstore.bundle.v0.3+json","subject":"wrong"}\n' > "$output"; exit 0 ;;
    oversized-verification) truncate -s 1048577 "$output"; exit 0 ;;
    unverified-verification) printf '{"mediaType":"application/vnd.dev.sigstore.bundle.v0.3+json","verified":false}\n' > "$output"; exit 0 ;;
  esac
fi
if [[ $name == docmost-local-mcp-* ]]; then
  case ${TEST_SCENARIO:-success} in
    partial) printf 'partial' > "$output"; exit 18 ;;
    oversized) truncate -s 134217729 "$output"; exit 0 ;;
    mismatch) printf 'tampered binary' > "$output"; exit 0 ;;
  esac
fi
if [[ $name == release-manifest.json ]]; then
  case ${TEST_SCENARIO:-success} in
    wrong-commit) jq '.source.commit = "ffffffffffffffffffffffffffffffffffffffff"' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
    wrong-version) jq '.version = "v9.9.9"' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
    wrong-product) jq '.product.name = "rmcp"' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
    wrong-product-version) jq '.product.version = "0.6.4"' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
    duplicate-asset) jq '.artifacts += [.artifacts[0]]' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
    malformed-digest) jq '.artifacts[0].sha256 = "not-a-sha256"' "$FIXTURE_DIR/$name" > "$output"; exit 0 ;;
  esac
fi
cp "$FIXTURE_DIR/$name" "$output"
MOCK_CURL

cat > "$test_root/mock-bin/cosign" <<'MOCK_COSIGN'
#!/usr/bin/env bash
set -euo pipefail
case ${TEST_SCENARIO:-success} in
  provenance|malformed-verification|mismatched-verification|unverified-verification) exit 1 ;;
esac
args=" $* "
[[ $args == *" verify-blob "* ]]
[[ $args == *" --certificate-identity https://github.com/BonEvil/docmost-local-mcp/.github/workflows/release.yml@refs/tags/v1.2.3 "* ]]
[[ $args == *" --certificate-oidc-issuer https://token.actions.githubusercontent.com "* ]]
MOCK_COSIGN
chmod +x "$test_root/mock-bin/curl" "$test_root/mock-bin/cosign"

run_installer() {
  PATH="$test_root/mock-bin:$PATH" FIXTURE_DIR="$test_root/fixtures" TEST_SCENARIO=$1 COSIGN_BIN=cosign \
    bash "$repo_root/scripts/install-atlas.sh" \
      --version "$version" \
      --expected-commit "$commit" \
      --expected-sha256 "$digest" \
      --asset-name "$asset" \
      --install-path "$test_root/install/docmost-local-mcp"
}

printf 'unverified-existing-file\n' > "$test_root/install/docmost-local-mcp"
run_installer success >/dev/null
cmp "$test_root/fixtures/$asset" "$test_root/install/docmost-local-mcp"
[[ -x $test_root/install/docmost-local-mcp ]]

for scenario in \
  mismatch provenance partial oversized unapproved wrong-commit wrong-version wrong-product \
  wrong-product-version duplicate-asset malformed-digest \
  missing-verification malformed-verification mismatched-verification redirected-verification \
  oversized-verification unverified-verification
do
  printf 'known-good-before-failure\n' > "$test_root/install/docmost-local-mcp"
  if run_installer "$scenario" >/dev/null 2>&1; then
    echo "scenario unexpectedly succeeded: $scenario" >&2
    exit 1
  fi
  [[ $(cat "$test_root/install/docmost-local-mcp") == known-good-before-failure ]] || {
    echo "failed install replaced existing executable: $scenario" >&2
    exit 1
  }
  ! compgen -G "$test_root/install/.docmost-install.*" >/dev/null || {
    echo "failed install left staging files: $scenario" >&2
    exit 1
  }
done

echo "release installer success and rejection paths: ok"
