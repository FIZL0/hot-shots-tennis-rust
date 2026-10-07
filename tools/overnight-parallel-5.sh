#!/usr/bin/env bash
# tools/overnight-parallel.py with 5 sessions at once (worktrees s1..s5) instead of 3:
#     tmux new -s hst tools/overnight-parallel-5.sh
# Refuses while another runner is going: two runners would share worktrees s1..s3.
cd "$(dirname "$(realpath "$0")")/.."
pgrep -f '[o]vernight-parallel.py' >/dev/null && { echo "a parallel run is already going; wait for it to end"; exit 1; }
HST_SLOTS=5 exec tools/overnight-parallel.py "$@"
