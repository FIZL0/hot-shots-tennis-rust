#!/bin/bash
# tools/release.sh VERSION [notes.md] [publish] — a full release (v1.0.0, "OpenTennis 1.0.0"): set
# crates/hst's version to VERSION (a "Release vVERSION" commit if it changes), build Linux + Windows + the unsigned
# iOS IPA + the Android APK (tools/build.sh all; the APK via tools/apk.sh, signed with this machine's ~/.android/debug.keystore:
# keep that file, a phone only takes an update signed with the same key), sign SHA256SUMS with the minisign key (~/.config/hst/release.key, asks its password) and upload to GitHub:
#   opentennis-windows-x86_64.zip  opentennis-linux-x86_64.tar.gz  opentennis-ios.ipa  opentennis-android.apk   what players download
#   opentennis-windows-x86_64.exe  opentennis-linux-x86_64             the bare binaries the updater fetches (plan/B374.md)
#   SHA256SUMS  SHA256SUMS.sig
#   web.tar.gz                                                         the website, which pages.yml deploys as built here
# VERSION with a suffix (1.1.0-rc.1) is a pre-release, never Latest. notes.md is the "New in …" list (Markdown
# bullets); without it, RELEASE_NOTES.md (kept up to date as changes land; emptied and committed after the release),
# and if that has no bullets either, the commit subjects since the last v* tag (required for the first release). A draft unless
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
[ -n "$notes" ] || ! grep -q '^- ' RELEASE_NOTES.md || notes=RELEASE_NOTES.md
last=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || true)
[ -n "$notes" ] || [ -n "$last" ] || { echo "the first release needs a notes.md (no earlier v* tag to list commits from)" >&2; exit 1; }
[ -z "$notes" ] || [ -f "$notes" ] || { echo "no such notes file: $notes" >&2; exit 1; }
[ -f "$key" ] || { echo "no signing key at $key (minisign -G -p ~/.config/hst/release.pub -s $key)" >&2; exit 1; }
command -v minisign >/dev/null || { echo "minisign missing: sudo pacman -S minisign" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: the release is tagged at HEAD" >&2; exit 1; }
# `gh release view` can't see drafts (they have no tag yet); the list can
! gh release list -L 100 --json tagName -q '.[].tagName' | grep -qx "$tag" || { echo "$tag already has a release (a draft?): delete or publish it on GitHub first" >&2; exit 1; }

# the game reports its crates' version (lobby handshake, updater), so it must be the tag's: the workspace's
# `[workspace.package]` version, the one the game's crates share (P60)
if ! grep -qx "version = \"$version\"" Cargo.toml; then
    sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
    cargo metadata -q --offline --format-version 1 >/dev/null  # rewrites the game crates' entries in Cargo.lock
    git commit -q -m "Release $tag" Cargo.toml Cargo.lock
fi

# M97: prompt/token/commit counts for the website and donate screen; the ledger keeps transcripts Claude Code deletes
tools/dev_stats.py >/dev/null
git add research/dev-stats.json web/stats.json
git diff --cached --quiet || git commit -q -m "Dev stats for $tag" research/dev-stats.json web/stats.json

# B492: every player build drops the dev-only HST_* variables and debug logs (hst's `ship` feature)
for f in tools/build.sh tools/apk.sh tools/ios-ipa.sh; do
    grep -q 'features[ =].*ship' "$f" || { echo "$f doesn't build with --features ship (B492)" >&2; exit 1; }
done
# M22: one commit stamp for every platform (the online build id: players of one release meet, other builds are refused)
HST_COMMIT=$(git rev-parse HEAD | cut -c1-7)
HST_COMMIT=$HST_COMMIT HST_VERSION=$tag tools/build.sh all  # B374: update.rs compares this tag
out=dist/release
rm -rf "$out" && mkdir -p "$out"
tar -czf "$out/opentennis-linux-x86_64.tar.gz" -C dist/linux .
(cd dist/windows && zip -q "../release/opentennis-windows-x86_64.zip" opentennis.exe)
cp dist/linux/opentennis "$out/opentennis-linux-x86_64"
cp dist/windows/opentennis.exe "$out/opentennis-windows-x86_64.exe"
cp dist/ios/opentennis-ios.ipa "$out/opentennis-ios.ipa"
cp dist/opentennis.apk "$out/opentennis-android.apk"
# the website too: pages.yml deploys this release's web.tar.gz in ~1 min instead of building it for ~20 (opentennis.dev,
# no HST_SITE: its links are the main site's)
env -u HST_SITE HST_COMMIT=$HST_COMMIT tools/web.sh
tar -czf "$out/web.tar.gz" -C web .
(cd "$out" && sha256sum opentennis-* > SHA256SUMS)
minisign -S -s "$key" -m "$out/SHA256SUMS" -x "$out/SHA256SUMS.sig" -t "OpenTennis $tag"
# the game checks against the key built into it (update.rs): refuse a release it would turn down
minisign -V -P "$(sed -n 's/^pub const PUBLIC_KEY: &str = "\(.*\)";/\1/p' crates/hst/src/update.rs)" -m "$out/SHA256SUMS" -x "$out/SHA256SUMS.sig" -q \
    || { echo "SHA256SUMS.sig doesn't match update.rs's PUBLIC_KEY: wrong key?" >&2; exit 1; }

body=$out/notes.md
{
    [ -z "$pre" ] || { echo "> **Pre-release:** a test build of the next version."; echo; }
    echo "OpenTennis is a remaster of *Hot Shots Tennis* (PS2, 2007): the original's AI, physics, shots and timing, played on today's"
    echo "screens at any resolution and aspect ratio, with online play and character mods."
    echo
    echo "## Download"
    echo
    echo "| Platform | File | How to play |"
    echo "|---|---|---|"
    echo "| **Windows** 10/11 | [\`opentennis-windows-x86_64.zip\`](https://github.com/$repo/releases/download/$tag/opentennis-windows-x86_64.zip) | Unzip, then drop your ISO on \`opentennis.exe\` |"
    echo "| **Linux** / **Steam Deck** | [\`opentennis-linux-x86_64.tar.gz\`](https://github.com/$repo/releases/download/$tag/opentennis-linux-x86_64.tar.gz) | Unpack and open \`opentennis\`, then pick your ISO |"
    echo "| **iPhone** / **iPad** (iOS 15+) | [\`opentennis-ios.ipa\`](https://github.com/$repo/releases/download/$tag/opentennis-ios.ipa) | Unsigned: sign and install it yourself (AltStore, Sideloadly…), then put your ISO in Files → On My iPhone → OpenTennis. Touch controls or a controller |"
    echo "| **Android** 8+ | [\`opentennis-android.apk\`](https://github.com/$repo/releases/download/$tag/opentennis-android.apk) | Install it (allow unknown apps), then pick your ISO on first start |"
    echo "| **Browser** | [opentennis.dev](https://opentennis.dev/) | Nothing to install |"
    echo
    echo "## What you need"
    echo
    echo "Your own disc image of *Hot Shots Tennis* (USA, SCUS-97610). **No game data is included**, and the game reads"
    echo "your ISO on your own machine: nothing is uploaded."
    echo
    echo "## New in $version"
    echo
    if [ -n "$notes" ]; then sed '/^<!--/d' "$notes"; else git log --format='- %s' "$last..HEAD" --no-merges; fi
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
gh release create "$tag" --target "$(git rev-parse HEAD)" --title "OpenTennis $version" \
    --notes-file "$body" $pre $publish "$out"/opentennis-* "$out"/web.tar.gz "$out"/SHA256SUMS "$out"/SHA256SUMS.sig
if [ "$notes" = RELEASE_NOTES.md ]; then
    sed -i '/^<!--/!d' RELEASE_NOTES.md
    git commit -qm "Release notes: emptied after v$version" -- RELEASE_NOTES.md && git push -q origin HEAD
fi
echo "$tag: $([ "$publish" = --draft ] && echo "draft, publish it on GitHub" || echo published)"
