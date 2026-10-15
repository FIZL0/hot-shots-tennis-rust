#!/usr/bin/env bash
# tools/ship-local.sh [-w] — put HEAD's website live from this machine instead of waiting ~20 min for Actions: build
# the browser version here (tools/web.sh), push HEAD to main, upload web/ and deploy it through pages.yml's `prebuilt`
# input (download + deploy, ~1 min). -w watches the deploy. Executables are left to Actions: the push still starts the
# full pages run, which rebuilds the rolling "Beta" release (and deploys the same website again when it ends).
set -euo pipefail
cd "$(dirname "$0")/.."
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: what ships is HEAD" >&2; exit 1; }
git fetch -q origin main
git merge-base --is-ancestor origin/main HEAD || { echo "origin/main has commits HEAD lacks: pull first" >&2; exit 1; }
sha=$(git rev-parse HEAD)

tools/web.sh
mkdir -p dist
tar -czf dist/web.tar.gz -C web .

git push -q origin HEAD:main

# web/ for pages.yml to pick up; a prerelease, so it never becomes "latest"
gh release delete web-prebuilt --cleanup-tag -y 2>/dev/null || true
gh release create web-prebuilt --prerelease --latest=false --target "$sha" --title "Web build ${sha::7} (tools/ship-local.sh)" \
    --notes "The browser version as tools/ship-local.sh built it, for pages.yml to deploy. Not a game download." dist/web.tar.gz

gh workflow run pages.yml --ref main -f prebuilt=true
sleep 5
id=$(gh run list -w pages.yml -e workflow_dispatch -L 1 --json databaseId -q '.[0].databaseId')
echo "deploying ${sha::7} — run $id"
[ "${1:-}" = -w ] && gh run watch "$id" --exit-status && echo "live: https://opentennis.dev/"
