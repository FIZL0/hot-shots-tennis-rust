#!/usr/bin/env bash
# Launch Hot Shots Tennis in PCSX2 with PINE (memory IPC) on, so tools/pine.py can read live game state.
#   tools/pcsx2-hst.sh [pcsx2 args]   start (a no-op if this instance is up)
#   tools/pcsx2-hst.sh stop           close this instance only
#   tools/pcsx2-hst.sh status         is this instance up (pid); the game's own state: tools/pine.py
# HST_PCSX2=N (parallel runs, set by tools/overnight-parallel.py): copy N of the user's PCSX2 config,
# ../<repo>-slots/pcsx2/sN (own PINE slot 28011+N, save states, memory cards), seeing only virtual pad N.
# Unset: the user's own PCSX2 config, which never sees the parallel pads.
set -e
T=$(dirname "$(realpath "$0")")
N=${HST_PCSX2:-}
PID=${XDG_RUNTIME_DIR:-/tmp}/hst-pcsx2$N.pid  # pcsx2-qt hides its environment (file caps), so track it by pid
PAD=${XDG_RUNTIME_DIR:-/tmp}/hst-vpad$N.fifo.lock  # held by copy N's vpad.py serve (started below)
nopad() { [ -n "$N" ] && fuser -k -TERM "$PAD" >/dev/null 2>&1 || true; }  # set -e: a failed &&-list tail exits
mine() { p=$(cat "$PID" 2>/dev/null) && [ "$(ps -o comm= -p "$p")" = pcsx2-qt ] && echo "$p"; true; }
if [ "$1" = stop ]; then p=$(mine); [ -n "$p" ] && kill $p; nopad; exit 0; fi
if [ "$1" = status ]; then p=$(mine); echo "PCSX2 ${N:+copy $N }$([ -n "$p" ] && echo "running (pid $p)" || echo "not running")"; exit 0; fi
# anything else that isn't a pcsx2-qt flag would be opened as a file (an error dialog that blocks the copy)
case $1 in ''|-*) ;; *) echo "usage: pcsx2-hst.sh [stop | status | -pcsx2-qt-flags]" >&2; exit 2;; esac
[ -n "$(mine)" ] && { echo "PCSX2 ${N:+copy $N }already running"; exit 0; }
if [ -n "$N" ]; then
    export XDG_CONFIG_HOME="$(dirname "$(git -C "$T" rev-parse --path-format=absolute --git-common-dir)")-slots/pcsx2/s$N"
    [ -d "$XDG_CONFIG_HOME/PCSX2" ] || { echo "no PCSX2 copy at $XDG_CONFIG_HOME"; exit 1; }
    export SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT=$(printf '0x1209/0x%04x' $((0x5000 + N)))
    setsid "$T/vpad.py" serve >>"$XDG_CONFIG_HOME/PCSX2/logs/vpad.log" 2>&1 &  # exits by itself if pad N is up
    sleep 0.5
else
    export SDL_GAMECONTROLLER_IGNORE_DEVICES=$(for n in 1 2 3 4 5 6 7 8; do printf '0x1209/0x%04x,' $((0x5000 + n)); done)
fi
INI=${XDG_CONFIG_HOME:-$HOME/.config}/PCSX2/inis/PCSX2.ini
sed -i 's/^EnablePINE = false/EnablePINE = true/' "$INI"  # PCSX2 rewrites the ini on exit; keep PINE on
# copies: no keyboard (a new window takes focus, so the user's Space would pause it); pad N's Guide toggles pause,
# which pine.py presses to resume a paused copy
[ -n "$N" ] && sed -i -e 's/^Keyboard = true/Keyboard = false/' -e 's|^TogglePause = .*|TogglePause = SDL-0/Guide|' "$INI"
[ -z "$N" ] && { echo $$ >"$PID"; exec pcsx2-qt "$@" -- "$T/../Hot Shots Tennis (USA).iso"; }  # exec keeps the pid
# a copy's pad goes with it, however PCSX2 ends (stop, kill, crash)
pcsx2-qt "$@" -- "$T/../Hot Shots Tennis (USA).iso" & echo $! >"$PID"
wait $! || true
nopad
