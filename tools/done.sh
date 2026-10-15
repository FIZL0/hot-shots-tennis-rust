#!/usr/bin/env bash
# Wrap a task up in one call (no subagent): tools/done.sh <ID> "<commit message>" [--blocked "<why + what unblocks>"]
# Runs tools/audit.sh (no retail addresses), the tests and the sim's float lint; passing: ticks <ID> (moves it to plan/DONE.md) or, with --blocked, marks it [~]; then commits
# everything. Failing: nothing is ticked or committed — fix it, or commit what's solid and block it.
cd "$(dirname "$(realpath "$0")")/.."
[ $# = 2 ] || { [ $# = 4 ] && [ "$3" = --blocked ]; } || { sed -n 2,4p "$0"; exit 2; }
tools/audit.sh && tools/check.sh || exit 1
# the sim's #![deny(clippy::float_arithmetic)] modules (C2: float math through ps2::F / vu0::Vf); clippy only
# reports the deny-level lints here, every other lint stays off
cargo clippy -q -p hst-sim --lib --message-format=short -- -A clippy::all 2>&1 | grep -E 'error' | head -20 | grep . &&
  { echo "CLIPPY FAILED: plain f32 arithmetic in a converted sim module (use ps2::F / vu0::Vf)"; exit 1; }
if [ $# = 4 ]; then tools/ctx.py block "$1" "$4"; else tools/ctx.py tick "$1"; fi || exit 1
git add -A && git commit -qm "$2" && git log -1 --oneline
