#!/usr/bin/env bash
# Parallel unattended run (tools/parallel.py): ./parallel.sh [N] [p|d] inside tmux, e.g. tmux new -s hst ./parallel.sh 5 p
#   N  sessions at once (default 3)
#   p  priority: tasks exactly in PLAN.md order, the next open one to each free slot, files may overlap
#   d  different (default): the first open "Next" task leads, the others pick later tasks that touch different files
exec "$(dirname "$(realpath "$0")")/tools/parallel.py" "$@"
