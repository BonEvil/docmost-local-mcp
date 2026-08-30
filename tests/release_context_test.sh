#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

git -C "$test_root" init -q
git -C "$test_root" config user.email release-test@example.invalid
git -C "$test_root" config user.name release-test
printf 'reviewed\n' > "$test_root/reviewed.txt"
git -C "$test_root" add reviewed.txt
git -C "$test_root" commit -qm reviewed
reviewed=$(git -C "$test_root" rev-parse HEAD)
git -C "$test_root" branch protected-main
printf 'outside\n' > "$test_root/outside.txt"
git -C "$test_root" add outside.txt
git -C "$test_root" commit -qm outside
outside=$(git -C "$test_root" rev-parse HEAD)

write_evidence() {
  local type=$1 verified=$2 reason=$3 commit=$4 gate=$5
  jq -S -n \
    --arg type "$type" --argjson verified "$verified" --arg reason "$reason" \
    --arg commit "$commit" --arg gate "$gate" \
    '{schemaVersion:1,repository:"BonEvil/docmost-local-mcp",tag:"v1.2.3",tagObjectType:$type,
      signature:{verified:$verified,reason:$reason},tagCommit:$commit,
      securityGate:{name:"Integrated security and release gate",conclusion:"success",headSha:$gate}}' \
    > "$test_root/context.json"
}

run_check() {
  (cd "$test_root" && bash "$repo_root/scripts/verify-release-context.sh" context.json "$1" protected-main)
}

write_evidence tag true valid "$reviewed" "$reviewed"
run_check "$reviewed" >/dev/null

for scenario in lightweight unsigned outside missing-gate; do
  expected=$reviewed
  case $scenario in
    lightweight) write_evidence commit true valid "$reviewed" "$reviewed" ;;
    unsigned) write_evidence tag false unsigned "$reviewed" "$reviewed" ;;
    outside) write_evidence tag true valid "$outside" "$outside"; expected=$outside ;;
    missing-gate) write_evidence tag true valid "$reviewed" "$outside" ;;
  esac
  if run_check "$expected" >/dev/null 2>&1; then
    echo "release context unexpectedly accepted: $scenario" >&2
    exit 1
  fi
done

echo "release context success and rejection paths: ok"
