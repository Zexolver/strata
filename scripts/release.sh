#!/bin/bash
# usage: scripts/release.sh X.Y.Z "release notes"
# Tags the current commit, builds linux-aarch64 + x86_64 in CI, publishes a pre-release with the binaries.
set -euo pipefail
v="$1"; notes="$2"; repo=Zexolver/strata
git push -q origin main
git tag -a "v$v" -m "Strata v$v"; git push -q origin "v$v"
gh workflow run build.yml --repo $repo --ref "v$v"
sleep 10
run=$(gh run list --repo $repo --workflow build.yml --branch "v$v" --limit 1 --json databaseId -q '.[0].databaseId')
gh run watch "$run" --repo $repo --exit-status >/dev/null
rm -rf /tmp/dist && for a in linux-aarch64 linux-x86_64; do gh run download "$run" --repo $repo -n $a -D /tmp/dist; done
gh release create "v$v" /tmp/dist/* --repo $repo --verify-tag --prerelease --title "Strata v$v" --notes "$notes"
