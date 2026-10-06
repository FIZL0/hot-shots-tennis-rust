#!/usr/bin/env bash
# Unattended runner: keeps starting `claude -p "continue"` (= do the next open prompt in TODO.md) until no
# unchecked prompts remain. Any exit — finished prompt, error, usage limit — just waits and starts again.
#
#   tmux new -s hst tools/overnight.sh        # detach: Ctrl-b d · reattach: tmux attach -t hst
#   HST_MAX_TURNS=400 HST_PAUSE=60 tools/overnight.sh
#
# Runs with permission prompts bypassed: review the work in the morning (git log), and keep your backup.
set -u
cd "$(dirname "$(realpath "$0")")/.."
log=context/notes/overnight.log
mkdir -p "$(dirname "$log")"
turns=${HST_MAX_TURNS:-400}
pause=${HST_PAUSE:-60}
run=0
while grep -q '^- \[ \]' TODO.md; do
  run=$((run + 1))
  echo "=== run $run $(date -Is) — next: $(grep -m1 '^- \[ \]' TODO.md | cut -c1-100)" | tee -a "$log"
  claude -p "continue" --permission-mode bypassPermissions --max-turns "$turns" >>"$log" 2>&1
  echo "=== run $run exited $? at $(date -Is)" | tee -a "$log"
  sleep "$pause"
done
echo "=== all TODO prompts checked off at $(date -Is)" | tee -a "$log"
