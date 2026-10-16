#!/bin/bash
# tools/asset_sweep.sh OUT_DIR [N]  — Z2: run the game under HST_ASSET_LOG over every disc court (singles and doubles,
# every umpire and weather), every character in every costume, every installed mod (characters, courts, umpires), and the menus (HST_MASH), N runs
# at a time (default 2), each in a tools/shot.sh window; one log per run in OUT_DIR/<run>.tsv. Then:
#   target/release/texkeys "Hot Shots Tennis (USA).iso" > OUT_DIR/texkeys.tsv
#   research/assets_used.py OUT_DIR/texkeys.tsv OUT_DIR/*.tsv --pack mods/<pack>/textures
# HST_SWEEP=matches|costumes|mods|menus|paths runs one part only (paths, Z21: every HST_MENU screen with and without
# --original, pause → Settings by HST_KEYS, 3-set singles and court-7 doubles to the result screens; not in `all`). Matches are all-computer (HST_AUTOPLAY), 1 game a set,
# shot (and quit) after SECS (HST_SWEEP_SECS, 200) so most reach the result screens.
set -u
cd "$(dirname "$0")/.."
out=$(realpath -m "${1:?usage: tools/asset_sweep.sh OUT_DIR [N]}"); n=${2:-2}
mkdir -p "$out"
secs=${HST_SWEEP_SECS:-200}
part=${HST_SWEEP:-all}
jobs=()

run() { # name wait env… -- args…
    local name=$1 wait=$2; shift 2
    local envs=(); while [ "$1" != -- ]; do envs+=("$1"); shift; done; shift
    [ -s "$out/$name.png" ] && return
    while [ "$(jobs -rp | wc -l)" -ge "$n" ]; do sleep 2; done
    ( env HST_ASSET_LOG="$out/$name.tsv" HST_WAIT="$wait" "${envs[@]}" tools/shot.sh "$out/$name.png" "$@" >/dev/null 2>&1 \
        || echo "asset_sweep: $name: no shot" >&2 ) &
}

if [ "$part" = all ] || [ "$part" = matches ]; then
    # every court singles and doubles; characters and costumes rotate, as do umpires (0..4) and weathers (0..3)
    i=0
    for s in 1 2 3 4 5 6 7 8 9 10 11; do
        for mode in singles doubles; do
            c=$((i * 4 % 14)); chars="$c,$(((c + 1) % 14)),$(((c + 2) % 14)),$(((c + 3) % 14))"
            o=$((i % 10)); outfits="$o,$(((o + 3) % 10)),$(((o + 6) % 10)),$(((o + 9) % 10))"
            extra=(); [ $mode = singles ] && extra=(--singles)
            run "match-s$s-$mode" $((secs + 60)) HST_AUTOPLAY=1 HST_WEATHER=$((i % 4)) -- \
                --stage "$s" --court "$s" --play "${extra[@]}" --sets 1 --games 1 --umpire $((i % 5)) \
                --chars "$chars" --outfits "$outfits" --shot-at "$secs"
            i=$((i + 1))
        done
    done
fi

if [ "$part" = all ] || [ "$part" = costumes ]; then
    # every character in every costume, four at a time, a few seconds of a match each
    for o in 0 1 2 3 4 5 6 7 8 9; do
        for c in 0 4 8 12; do
            chars="$c,$(((c + 1) % 14)),$(((c + 2) % 14)),$(((c + 3) % 14))"
            run "costume-c$c-o$o" 90 HST_AUTOPLAY=1 -- --stage $((o + 1)) --court $((o + 1)) --play --chars "$chars" \
                --outfits "$o,$o,$o,$o" --shot-at 25
        done
    done
fi

if [ "$part" = all ] || [ "$part" = mods ]; then
    # every installed mod in a short match: characters in seat 1 (its first costume), courts on their layout, umpires
    i=0
    for d in mods/*/ mods/hst-mods/*/; do
        d=${d%/}; [ -f "$d/mod.json" ] || continue
        kind=$(grep -o '"kind": *"[a-z]*"' "$d/mod.json" | sed 's/.*"\([a-z]*\)"$/\1/')
        case $kind in
            textures) continue ;;
            court) a=(--court-mod "$d") ;;
            umpire) a=(--stage 1 --court 1 --umpire-mod "$d") ;;
            *) a=(--stage $((i % 11 + 1)) --court $((i % 11 + 1)) --mod "$d" --mod-slot 0) ;;
        esac
        run "mod-$(basename "$d")" 90 HST_AUTOPLAY=1 -- --play "${a[@]}" --shot-at 30
        i=$((i + 1))
    done
fi

if [ "$part" = all ] || [ "$part" = menus ]; then
    # the front-end: seeded random key mashing through every menu (HST_MASH), several seeds
    for seed in 1 2 3 4 5 6; do
        run "menus-$seed" 200 HST_MASH="150,$seed,3" -- --shot-at 149
    done
fi
if [ "$part" = paths ]; then
    # every menu screen straight from HST_MENU (the mash reaches few), remastered and --original
    for o in "" original; do
        for m in press main rematch settings graphics advanced sound controls mode assign chars chars4 attrs names coms \
            confirm confirm-random confirm-court courts umpires sets mods extras online lobbies; do
            run "menu-$m${o:+-$o}" 40 HST_MENU=$m -- ${o:+--$o} --shot-at 8
        done
    done
    # pause → Settings (row 2), a tab across, back out, in both looks
    for o in "" original; do
        run "pause${o:+-$o}" 90 HST_AUTOPLAY=1 HST_KEYS="20:Escape 23:ArrowDown 25:Enter 28:ArrowRight 31:ArrowRight 34:Escape 37:Escape" -- \
            ${o:+--$o} --stage 3 --court 3 --play --singles --shot-at 40
    done
    # set pop-ups and the result screens: best of three sets of one game, singles on court 1, doubles on court 7
    run "sets3-s1-singles" $((secs * 3 + 60)) HST_AUTOPLAY=1 HST_WEATHER=0 -- --stage 1 --court 1 --play --singles --sets 3 --games 1 --shot-at $((secs * 3))
    run "sets3-s7-doubles" $((secs * 3 + 60)) HST_AUTOPLAY=1 HST_WEATHER=1 -- --stage 7 --court 7 --play --sets 3 --games 1 --original --shot-at $((secs * 3))
fi
wait
echo "asset_sweep: $(ls "$out"/*.tsv 2>/dev/null | wc -l) logs in $out"
