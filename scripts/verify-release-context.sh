#!/usr/bin/env bash
set -euo pipefail

fail() { echo "verify-release-context: $*" >&2; exit 1; }

[[ $# -eq 3 ]] || fail "usage: $0 RELEASE_CONTEXT_JSON EXPECTED_COMMIT PROTECTED_BRANCH_REF"
evidence=$1
expected_commit=$2
protected_branch=$3

command -v jq >/dev/null || fail "jq is required"
[[ -s $evidence ]] || fail "release-context evidence is missing"
[[ $expected_commit =~ ^[0-9a-f]{40}$ ]] || fail "expected commit is not a full lowercase SHA"

jq -e '
  .schemaVersion == 1 and
  (.repository | type == "string" and length > 0) and
  (.tag | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+([.-][A-Za-z0-9.-]+)?$")) and
  .tagObjectType == "tag" and
  .signature.verified == true and
  .signature.reason == "valid" and
  (.tagCommit | test("^[0-9a-f]{40}$")) and
  .securityGate.name == "Integrated security and release gate" and
  .securityGate.conclusion == "success" and
  (.securityGate.headSha | test("^[0-9a-f]{40}$"))
' "$evidence" >/dev/null || fail "tag signature or security-gate evidence is invalid"

tag_commit=$(jq -r '.tagCommit' "$evidence")
gate_commit=$(jq -r '.securityGate.headSha' "$evidence")
[[ $tag_commit == "$expected_commit" ]] || fail "signed tag commit does not match the release commit"
[[ $gate_commit == "$expected_commit" ]] || fail "security gate did not pass for the release commit"

git rev-parse --verify --quiet "$protected_branch^{commit}" >/dev/null || \
  fail "protected branch ref is unavailable"
git merge-base --is-ancestor "$expected_commit" "$protected_branch" || \
  fail "release commit is outside protected branch ancestry"

echo "signed tag, protected ancestry, and exact-commit security gate: ok"
