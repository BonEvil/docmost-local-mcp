#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY="BonEvil/docmost-local-mcp"
readonly OIDC_ISSUER="https://token.actions.githubusercontent.com"
readonly MAX_BINARY_BYTES=$((128 * 1024 * 1024))
readonly MAX_METADATA_BYTES=$((1024 * 1024))

die() { echo "install-atlas: $*" >&2; exit 1; }

usage() {
  cat >&2 <<'EOF'
usage: install-atlas.sh --version vX.Y.Z --expected-commit FULL_SHA \
  --expected-sha256 SHA256 --install-path ABSOLUTE_PATH [--asset-name NAME]

Requires curl, jq, SHA-256 tooling, and cosign 3.0.3. The release manifest must
be signed by this fork's release workflow for the requested tag.
EOF
  exit 64
}

url_host() {
  local url=$1 authority
  [[ $url =~ ^https://([^/]+)/ ]] || return 1
  authority=${BASH_REMATCH[1]}
  [[ $authority != *"@"* && $authority != *":"* ]] || return 1
  printf '%s\n' "$authority" | tr '[:upper:]' '[:lower:]'
}

approved_url() {
  local host
  host=$(url_host "$1") || return 1
  case "$host" in
    github.com|objects.githubusercontent.com|release-assets.githubusercontent.com) return 0 ;;
    *) return 1 ;;
  esac
}

download_verified_transport() {
  local url=$1 output=$2 max_bytes=$3 redirects=0 headers status location
  headers="${output}.headers"
  while :; do
    approved_url "$url" || die "refusing unapproved or non-HTTPS URL: $url"
    : > "$headers"
    if ! curl --silent --show-error --proto '=https' --tlsv1.2 \
      --connect-timeout 15 --max-time 300 --max-redirs 0 \
      --max-filesize "$max_bytes" --dump-header "$headers" --output "$output" "$url"; then
      rm -f "$output" "$headers"
      die "download was partial, oversized, or failed: $url"
    fi
    status=$(awk 'toupper($1) ~ /^HTTP\// {code=$2} END {print code}' "$headers")
    if [[ $status == 200 ]]; then
      [[ -f $output ]] || die "download produced no file: $url"
      [[ $(wc -c < "$output") -le $max_bytes ]] || die "download exceeded size limit: $url"
      rm -f "$headers"
      return
    fi
    if [[ $status =~ ^30[12378]$ ]]; then
      (( redirects < 5 )) || die "too many redirects"
      location=$(awk '/^[Ll][Oo][Cc][Aa][Tt][Ii][Oo][Nn]:/ {sub(/^[^:]*:[[:space:]]*/, ""); sub(/\r$/, ""); print; exit}' "$headers")
      [[ -n $location ]] || die "redirect omitted Location"
      approved_url "$location" || die "redirect target is not an approved HTTPS host: $location"
      url=$location
      redirects=$((redirects + 1))
      continue
    fi
    die "unexpected HTTP status ${status:-unknown}: $url"
  done
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else die "sha256sum or shasum is required"
  fi
}

default_asset() {
  local os arch ext=""
  case "$(uname -s)" in Darwin) os=darwin ;; Linux) os=linux ;; *) die "unsupported operating system" ;; esac
  case "$(uname -m)" in x86_64|amd64) arch=x64 ;; arm64|aarch64) arch=arm64 ;; *) die "unsupported architecture" ;; esac
  printf 'docmost-local-mcp-%s-%s%s\n' "$os" "$arch" "$ext"
}

version="" expected_commit="" expected_sha256="" install_path="" asset_name=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) version=${2:-}; shift 2 ;;
    --expected-commit) expected_commit=${2:-}; shift 2 ;;
    --expected-sha256) expected_sha256=${2:-}; shift 2 ;;
    --install-path) install_path=${2:-}; shift 2 ;;
    --asset-name) asset_name=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done

[[ $version =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]] || usage
[[ $expected_commit =~ ^[0-9a-f]{40}$ ]] || usage
[[ $expected_sha256 =~ ^[0-9a-f]{64}$ ]] || usage
[[ $install_path == /* && $install_path != */ ]] || usage
asset_name=${asset_name:-$(default_asset)}
[[ $asset_name =~ ^docmost-local-mcp-(darwin|linux)-(arm64|x64)$ ]] || die "invalid asset name"

command -v curl >/dev/null || die "curl is required"
command -v jq >/dev/null || die "jq is required"
cosign_bin=${COSIGN_BIN:-cosign}
command -v "$cosign_bin" >/dev/null || die "cosign is required"

install_dir=$(dirname "$install_path")
[[ -d $install_dir ]] || die "install directory does not exist: $install_dir"
work_dir=$(mktemp -d "$install_dir/.docmost-install.XXXXXX") || die "cannot create installation staging directory"
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM

base_url="https://github.com/$REPOSITORY/releases/download/$version"
artifact="$work_dir/$asset_name"
manifest="$work_dir/release-manifest.json"
bundle="$work_dir/release-manifest.sigstore.json"
download_verified_transport "$base_url/$asset_name" "$artifact" "$MAX_BINARY_BYTES"
download_verified_transport "$base_url/release-manifest.json" "$manifest" "$MAX_METADATA_BYTES"
download_verified_transport "$base_url/release-manifest.sigstore.json" "$bundle" "$MAX_METADATA_BYTES"

identity="https://github.com/$REPOSITORY/.github/workflows/release.yml@refs/tags/$version"
"$cosign_bin" verify-blob \
  --bundle "$bundle" \
  --certificate-identity "$identity" \
  --certificate-oidc-issuer "$OIDC_ISSUER" \
  "$manifest" >/dev/null || die "release provenance verification failed"

manifest_commit=$(jq -er '.source.commit' "$manifest") || die "manifest has no source commit"
manifest_version=$(jq -er '.version' "$manifest") || die "manifest has no version"
manifest_product_name=$(jq -er '.product.name' "$manifest") || die "manifest has no product name"
manifest_product_version=$(jq -er '.product.version' "$manifest") || die "manifest has no product version"
manifest_digest=$(jq -er --arg name "$asset_name" \
  '.artifacts | map(select(.name == $name)) | if length == 1 then .[0].sha256 else error("asset must appear exactly once") end' \
  "$manifest") || die "manifest does not identify exactly one requested artifact"

[[ $manifest_commit == "$expected_commit" ]] || die "manifest commit does not match reviewed commit"
[[ $manifest_version == "$version" ]] || die "manifest version does not match requested release"
[[ $manifest_product_name == docmost-local-mcp ]] || die "manifest identifies the wrong product"
[[ "v$manifest_product_version" == "$version" ]] || die "manifest product version does not match requested release"
[[ $manifest_digest == "$expected_sha256" ]] || die "manifest digest does not match reviewed digest"
actual_digest=$(sha256_file "$artifact")
[[ $actual_digest == "$expected_sha256" ]] || die "artifact digest mismatch"

chmod 0755 "$artifact"
mv -f "$artifact" "$install_path"
echo "installed $install_path from $REPOSITORY@$expected_commit ($actual_digest)"
