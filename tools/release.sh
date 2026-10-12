#!/bin/bash
# tools/release.sh [notes.md] [publish] — build Linux + Windows (tools/build.sh all), pack them like the earlier
# releases (hst-linux-x86_64.tar.gz, hst-windows-x86_64.zip) and upload them as the next alpha
# (v0.1.0-alpha.N+1, "Alpha N+1") on GitHub. notes.md is the "New in Alpha N" list; without it, the commit subjects
# since the last tag. A draft unless `publish` is given: check it on GitHub and publish there.
set -e
cd "$(dirname "$0")/.."
notes= publish=--draft
for a; do [ "$a" = publish ] && publish=--latest || notes=$a; done

last=$(git describe --tags --abbrev=0 --match 'v0.1.0-alpha.*')
n=$(( ${last##*.} + 1 ))
tag=v0.1.0-alpha.$n
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: the release is tagged at HEAD" >&2; exit 1; }

tools/build.sh all
rm -f dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip
tar -czf dist/hst-linux-x86_64.tar.gz -C dist/linux .
(cd dist/windows && zip -q ../hst-windows-x86_64.zip hst.exe)

body=$(mktemp)
{
    echo "**Alpha build: expect bugs and missing pieces.**"
    echo
    echo "A Rust/Bevy remaster of Hot Shots Tennis (PS2). **No game data is included.** You need your own copy of *Hot Shots Tennis (USA)* (SCUS-97610) as an ISO."
    echo
    echo "Or play in the browser: https://fizl0.github.io/hot-shots-tennis-rust/"
    echo
    echo "### New in Alpha $n"
    if [ -n "$notes" ]; then cat "$notes"; else git log --format='- %s' "$last..HEAD" --no-merges; fi
} > "$body"

git push -q origin HEAD
gh release create "$tag" --target "$(git rev-parse HEAD)" --title "Alpha $n" --notes-file "$body" $publish \
    dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip
rm "$body"
