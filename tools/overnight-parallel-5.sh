#!/usr/bin/env bash
# tools/overnight-parallel.py with 5 sessions at once (worktrees s1..s5) instead of 3:
#     tmux new -s hst tools/overnight-parallel-5.sh
cd "$(dirname "$(realpath "$0")")/.."
HST_SLOTS=5 exec tools/overnight-parallel.py "$@"
