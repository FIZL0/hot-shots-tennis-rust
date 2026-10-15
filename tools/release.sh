#!/bin/bash
# tools/release.sh [VERSION] [notes.md] [publish] — build Linux + Windows (tools/build.sh all), pack them like the
# earlier releases (hst-linux-x86_64.tar.gz, hst-windows-x86_64.zip) and upload them to GitHub.
#   no VERSION        the next alpha: v0.1.0-alpha.N+1, "Alpha N+1"
#   VERSION (1.0.0)   a full release: v1.0.0, "Version 1.0.0", no alpha warning
# notes.md is the "New in …" list; without it, the commit subjects since the last v* tag. A draft unless `publish`
# is given: check it on GitHub and publish there. Refuses a tag that already has a release (drafts included).
# The rolling "Latest alpha" (tag `alpha`) is not made here: every pages.yml run on main rebuilds it.
# B374: the builds carry their tag (HST_VERSION, `update.rs`) and the release also gets the bare binaries
# (hst-linux-x86_64, hst-windows-x86_64.exe) and their SHA256SUMS, signed with minisign (SHA256SUMS.sig), which the
# game's updater downloads and checks. The secret key stays outside git; without it this refuses to publish. Once:
#   minisign -G -p ~/.config/hst/release.pub -s ~/.config/hst/release.key   # and the .pub's key line into update.rs
set -e
cd "$(dirname "$0")/.."
notes= publish=--draft version=
for a; do
    case $a in
        publish) publish=--latest ;;
        v[0-9]*|[0-9]*) version=${a#v} ;;
        *) notes=$a ;;
    esac
done

last=$(git describe --tags --abbrev=0 --match 'v[0-9]*')
if [ -n "$version" ]; then
    tag=v$version title="Version $version"
else
    n=$(git tag -l 'v0.1.0-alpha.*' | sed 's/.*\.//' | sort -n | tail -1)
    n=$(( n + 1 ))
    tag=v0.1.0-alpha.$n title="Alpha $n"
fi
key=~/.config/hst/release.key
[ -f "$key" ] || { echo "no $key: make the release key first (minisign -G, see the top of this script)" >&2; exit 1; }
command -v minisign >/dev/null || { echo "release.sh needs minisign (sudo pacman -S minisign)" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: the release is tagged at HEAD" >&2; exit 1; }
# `gh release view` can't see drafts (they have no tag yet); the list can
! gh release list -L 100 --json tagName -q '.[].tagName' | grep -qx "$tag" || { echo "$tag already has a release (a draft?): delete or publish it on GitHub first" >&2; exit 1; }

HST_VERSION=$tag tools/build.sh all
rm -f dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip dist/hst-linux-x86_64 dist/hst-windows-x86_64.exe dist/SHA256SUMS dist/SHA256SUMS.sig
tar -czf dist/hst-linux-x86_64.tar.gz -C dist/linux .
(cd dist/windows && zip -q ../hst-windows-x86_64.zip hst.exe)
cp dist/linux/hst dist/hst-linux-x86_64
cp dist/windows/hst.exe dist/hst-windows-x86_64.exe
(cd dist && sha256sum hst-linux-x86_64 hst-windows-x86_64.exe hst-linux-x86_64.tar.gz hst-windows-x86_64.zip > SHA256SUMS)
minisign -S -s "$key" -m dist/SHA256SUMS -x dist/SHA256SUMS.sig -t "Hot Shots Tennis $tag"
# the game checks against the key built into it: refuse a release it would turn down
minisign -V -P "$(sed -n 's/^pub const PUBLIC_KEY: &str = "\(.*\)";/\1/p' crates/hst/src/update.rs)" -m dist/SHA256SUMS -x dist/SHA256SUMS.sig -q \
    || { echo "SHA256SUMS.sig doesn't match update.rs's PUBLIC_KEY: wrong key?" >&2; exit 1; }

body=$(mktemp)
{
    [ -n "$version" ] || { echo "**Alpha build: expect bugs and missing pieces.**"; echo; }
    echo "A Rust/Bevy remaster of Hot Shots Tennis (PS2). **No game data is included.** You need your own copy of *Hot Shots Tennis (USA)* (SCUS-97610) as an ISO."
    echo
    echo "Or play in the browser: https://opentennis.dev/"
    echo
    echo "### New in $title"
    if [ -n "$notes" ]; then cat "$notes"; else git log --format='- %s' "$last..HEAD" --no-merges; fi
} > "$body"

git push -q origin HEAD
gh release create "$tag" --target "$(git rev-parse HEAD)" --title "$title" --notes-file "$body" $publish \
    dist/hst-linux-x86_64.tar.gz dist/hst-windows-x86_64.zip \
    dist/hst-linux-x86_64 dist/hst-windows-x86_64.exe dist/SHA256SUMS dist/SHA256SUMS.sig
rm "$body"
