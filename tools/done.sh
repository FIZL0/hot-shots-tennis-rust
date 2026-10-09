#!/usr/bin/env bash
# Wrap a task up in one call (no subagent): tools/done.sh <ID> "<commit message>" [--blocked "<why + what unblocks>"]
# Runs the tests; passing: ticks <ID> (moves it to plan/DONE.md) or, with --blocked, marks it [~]; then commits
# everything. Failing: nothing is ticked or committed — fix it, or commit what's solid and block it.
cd "$(dirname "$(realpath "$0")")/.."
[ $# = 2 ] || { [ $# = 4 ] && [ "$3" = --blocked ]; } || { sed -n 2,4p "$0"; exit 2; }
tools/check.sh || exit 1
if [ $# = 4 ]; then tools/ctx.py block "$1" "$4"; else tools/ctx.py tick "$1"; fi || exit 1
git add -A && git commit -qm "$2" && git log -1 --oneline
