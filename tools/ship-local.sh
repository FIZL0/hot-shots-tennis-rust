#!/usr/bin/env bash
# tools/ship-local.sh [-w] — ship HEAD from this machine instead of waiting ~20 min for Actions: build the browser
# version (tools/web.sh) and Linux + Windows (tools/build.sh all) here, push HEAD to main, cancel the pages builds,
# replace the rolling "Latest alpha" release with these builds and deploy web/ through pages.yml's `prebuilt` input
# (download + deploy, ~1 min). -w watches the deploy. Pushes to main still ship by themselves through pages.yml.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: what ships is HEAD" >&2; exit 1; }
git fetch -q origin main
git merge-base --is-ancestor origin/main HEAD || { echo "origin/main has commits HEAD lacks: pull first" >&2; exit 1; }
sha=$(git rev-parse HEAD)

tools/web.sh
tools/build.sh all
rm -f dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip dist/web.tar.gz
tar -czf dist/hst-linux-x86_64.tar.gz -C dist/linux .
(cd dist/windows && zip -q ../hst-windows-x86_64.zip hst.exe)
tar -czf dist/web.tar.gz -C web .

git push -q origin HEAD:main

# web/ for pages.yml to pick up; a prerelease, so it never becomes "latest"
gh release delete web-prebuilt --cleanup-tag -y 2>/dev/null || true
gh release create web-prebuilt --prerelease --latest=false --target "$sha" --title "Web build ${sha::7} (tools/ship-local.sh)" \
    --notes "The browser version as tools/ship-local.sh built it, for pages.yml to deploy. Not a game download." dist/web.tar.gz

# same release as pages.yml's alpha job makes, with this machine's builds
gh release delete alpha --cleanup-tag -y 2>/dev/null || true
printf '%s\n\n%s\n\n%s\n' "**Rolling alpha: rebuilt on every change to main (${sha::7}). Expect bugs.**" \
    "No game data is included: you need your own *Hot Shots Tennis (USA)* (SCUS-97610) ISO." \
    "Browser version: https://opentennis.dev/" > dist/alpha-notes.md
gh release create alpha --latest --target "$sha" --title "Latest alpha (${sha::7})" --notes-file dist/alpha-notes.md \
    dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip

# the push started a full build of the same commit: not needed now
for s in in_progress queued pending waiting; do
    gh run list -w pages.yml -s "$s" --json databaseId -q '.[].databaseId' | xargs -r -n1 gh run cancel
done
gh workflow run pages.yml --ref main -f prebuilt=true
sleep 5
id=$(gh run list -w pages.yml -e workflow_dispatch -L 1 --json databaseId -q '.[0].databaseId')
echo "deploying ${sha::7} — run $id"
[ "${1:-}" = -w ] && gh run watch "$id" --exit-status && echo "live: https://opentennis.dev/"
