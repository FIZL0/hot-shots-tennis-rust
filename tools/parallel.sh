#!/usr/bin/env bash
# Parallel unattended run (tools/parallel.py): tools/parallel.sh [N] [p|d] [f] inside tmux, e.g. tmux new -s hst tools/parallel.sh 5 p
#   N  sessions at once (default 3)
#   p  priority: tasks exactly in plan/TODO.md order, the next open one to each free slot, files may overlap
#   d  different (default): the first open "Next" task leads, the others pick later tasks that touch different files
#   f  force: keep starting tasks past 90% weekly usage (WEEKLY_MAX in parallel.py)
exec "$(dirname "$(realpath "$0")")/parallel.py" "$@"
