#!/bin/bash
# tools/release.sh VERSION [notes.md] [publish] — a full release (v1.0.0, "Hot Shots Tennis Remastered 1.0.0"): set
# crates/hst's version to VERSION (a "Release vVERSION" commit if it changes), build Linux + Windows + the unsigned
# iOS IPA (tools/build.sh all), sign SHA256SUMS with the minisign key (~/.config/hst/release.key, asks its password) and upload to GitHub:
#   hst-windows-x86_64.zip  hst-linux-x86_64.tar.gz  hst-ios.ipa   what players download
#   hst-windows-x86_64.exe  hst-linux-x86_64             the bare binaries the updater fetches (plan/B374.md)
#   SHA256SUMS  SHA256SUMS.sig
# VERSION with a suffix (1.1.0-rc.1) is a pre-release, never Latest. notes.md is the "New in …" list (Markdown
# bullets); without it, the commit subjects since the last v* tag (required for the first release). A draft unless
# `publish` is given: check it on GitHub and publish there. Publishing puts the release on opentennis.dev when the repo
# variable SITE_FROM is `release` (pages.yml). Every push to main is a beta (pages.yml), not made here.
set -e
cd "$(dirname "$0")/.."
key=~/.config/hst/release.key
pub=RWTW5uzveKqmVluF6+6vYS5Uq/H7St1d9WMjyxVxXGpJAbr8+sT6HimM
repo=FIZL0/opentennis
notes= publish=--draft version=
for a; do
    case $a in
        publish) publish=--latest ;;
        v[0-9]*|[0-9]*) version=${a#v} ;;
        *) notes=$a ;;
    esac
done
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "usage: tools/release.sh VERSION [notes.md] [publish]  (VERSION like 1.0.0)" >&2; exit 1; }
tag=v$version
pre=
[[ $version == *-* ]] && { pre=--prerelease; [ "$publish" = --draft ] || publish=--latest=false; }
last=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || true)
[ -n "$notes" ] || [ -n "$last" ] || { echo "the first release needs a notes.md (no earlier v* tag to list commits from)" >&2; exit 1; }
[ -z "$notes" ] || [ -f "$notes" ] || { echo "no such notes file: $notes" >&2; exit 1; }
[ -f "$key" ] || { echo "no signing key at $key (minisign -G -p ~/.config/hst/release.pub -s $key)" >&2; exit 1; }
command -v minisign >/dev/null || { echo "minisign missing: sudo pacman -S minisign" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: the release is tagged at HEAD" >&2; exit 1; }
# `gh release view` can't see drafts (they have no tag yet); the list can
! gh release list -L 100 --json tagName -q '.[].tagName' | grep -qx "$tag" || { echo "$tag already has a release (a draft?): delete or publish it on GitHub first" >&2; exit 1; }

# the game reports its crate version (lobby handshake, updater), so it must be the tag's
if ! grep -qx "version = \"$version\"" crates/hst/Cargo.toml; then
    sed -i "0,/^version = \".*\"/s//version = \"$version\"/" crates/hst/Cargo.toml
    cargo metadata -q --offline --format-version 1 >/dev/null  # rewrites hst's entry in Cargo.lock
    git commit -q -m "Release $tag" crates/hst/Cargo.toml Cargo.lock
fi

# M97: prompt/token/commit counts for the website and donate screen; the ledger keeps transcripts Claude Code deletes
tools/dev_stats.py >/dev/null
git add research/dev-stats.json web/stats.json
git diff --cached --quiet || git commit -q -m "Dev stats for $tag" research/dev-stats.json web/stats.json

HST_VERSION=$tag tools/build.sh all  # B374: update.rs compares this tag
out=dist/release
rm -rf "$out" && mkdir -p "$out"
tar -czf "$out/hst-linux-x86_64.tar.gz" -C dist/linux .
(cd dist/windows && zip -q "../release/hst-windows-x86_64.zip" hst.exe)
cp dist/linux/hst "$out/hst-linux-x86_64"
cp dist/windows/hst.exe "$out/hst-windows-x86_64.exe"
cp dist/ios/hst-ios.ipa "$out/hst-ios.ipa"
(cd "$out" && sha256sum hst-* > SHA256SUMS)
minisign -S -s "$key" -m "$out/SHA256SUMS" -x "$out/SHA256SUMS.sig" -t "Hot Shots Tennis Remastered $tag"
# the game checks against the key built into it (update.rs): refuse a release it would turn down
minisign -V -P "$(sed -n 's/^pub const PUBLIC_KEY: &str = "\(.*\)";/\1/p' crates/hst/src/update.rs)" -m "$out/SHA256SUMS" -x "$out/SHA256SUMS.sig" -q \
    || { echo "SHA256SUMS.sig doesn't match update.rs's PUBLIC_KEY: wrong key?" >&2; exit 1; }

body=$out/notes.md
{
    [ -z "$pre" ] || { echo "> **Pre-release:** a test build of the next version."; echo; }
    echo "A remaster of *Hot Shots Tennis* (PS2, 2007): the original's AI, physics, shots and timing, played on today's"
    echo "screens at any resolution and aspect ratio, with online play and character mods."
    echo
    echo "## Download"
    echo
    echo "| Platform | File | How to play |"
    echo "|---|---|---|"
    echo "| **Windows** 10/11 | [\`hst-windows-x86_64.zip\`](https://github.com/$repo/releases/download/$tag/hst-windows-x86_64.zip) | Unzip, then drop your ISO on \`hst.exe\` |"
    echo "| **Linux** / **Steam Deck** | [\`hst-linux-x86_64.tar.gz\`](https://github.com/$repo/releases/download/$tag/hst-linux-x86_64.tar.gz) | Unpack, then \`./hst \"Hot Shots Tennis (USA).iso\"\` |"
    echo "| **iPhone** / **iPad** (iOS 15+) | [\`hst-ios.ipa\`](https://github.com/$repo/releases/download/$tag/hst-ios.ipa) | Unsigned: sign and install it yourself (AltStore, Sideloadly…), then put your ISO in Files → On My iPhone → Hot Shots. Needs a controller |"
    echo "| **Browser** | [opentennis.dev](https://opentennis.dev/) | Nothing to install |"
    echo
    echo "## What you need"
    echo
    echo "Your own disc image of *Hot Shots Tennis* (USA, SCUS-97610). **No game data is included**, and the game reads"
    echo "your ISO on your own machine: nothing is uploaded."
    echo
    echo "## New in $version"
    echo
    if [ -n "$notes" ]; then cat "$notes"; else git log --format='- %s' "$last..HEAD" --no-merges; fi
    echo
    echo "## Verify the download"
    echo
    echo "\`SHA256SUMS\` is signed with the project's [minisign](https://jedisct1.github.io/minisign/) key:"
    echo
    echo '```'
    echo "minisign -Vm SHA256SUMS -P $pub && sha256sum -c --ignore-missing SHA256SUMS"
    echo '```'
    echo
    echo "Want the newest changes before the next release? Try the [beta](https://github.com/$repo/releases/tag/beta) or [beta.opentennis.dev](https://beta.opentennis.dev/)."
} > "$body"
git push -q origin HEAD
gh release create "$tag" --target "$(git rev-parse HEAD)" --title "Hot Shots Tennis Remastered $version" \
    --notes-file "$body" $pre $publish "$out"/hst-* "$out"/SHA256SUMS "$out"/SHA256SUMS.sig
echo "$tag: $([ "$publish" = --draft ] && echo "draft, publish it on GitHub" || echo published)"
