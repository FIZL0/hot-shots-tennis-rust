#!/usr/bin/env bash
# Wrap a task up in one call (no subagent): tools/done.sh <ID> "<commit message>" [--blocked "<why + what unblocks>"] [-- <paths>]
# Runs tools/audit.sh (no retail addresses), the tests and the sim's float lint; passing: ticks <ID> (moves it to plan/DONE.md) or, with --blocked, marks it [~]; then commits
# everything (in a worktree) or only plan/ + <paths> (main checkout, where other sessions' edits sit). Failing: nothing is ticked or committed.
cd "$(dirname "$(realpath "$0")")/.."
paths=(); for ((i = 1; i <= $#; i++)); do [ "${!i}" = -- ] && { paths=("${@:i+1}"); set -- "${@:1:i-1}"; break; }; done
[ $# = 2 ] || { [ $# = 4 ] && [ "$3" = --blocked ]; } || { sed -n 2,4p "$0"; exit 2; }
main=; [ "$(git rev-parse --git-dir)" = "$(git rev-parse --git-common-dir)" ] && main=1
[ "$main" ] && [ ${#paths[@]} = 0 ] && { echo "main checkout: name the paths you changed: tools/done.sh <ID> \"<msg>\" -- <paths> (git add -A would commit other sessions' edits)"; exit 2; }
tools/audit.sh && tools/check.sh || exit 1
# the sim's #![deny(clippy::float_arithmetic)] modules (C2: float math through ps2::F / vu0::Vf); clippy only
# reports the deny-level lints here, every other lint stays off
cargo clippy -q -p hst-sim --lib --message-format=short -- -A clippy::all 2>&1 | grep -E 'error' | head -20 | grep . &&
  { echo "CLIPPY FAILED: plain f32 arithmetic in a converted sim module (use ps2::F / vu0::Vf)"; exit 1; }
if [ $# = 4 ]; then tools/ctx.py block "$1" "$4"; else tools/ctx.py tick "$1"; fi || exit 1
if [ "$main" ]; then git add -- "${paths[@]}" && git commit -qm "$2" -- plan/TODO.md plan/DONE.md "${paths[@]}"
else git add -A && git commit -qm "$2"; fi && git log -1 --oneline
