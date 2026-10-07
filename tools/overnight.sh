#!/usr/bin/env bash
# Unattended runner: keeps starting `claude "continue"` (= do the next open prompt in PLAN.md) until no
# unchecked prompts remain. Any exit — finished prompt, error, usage limit — just waits and starts again.
# It's the normal TUI, so attach to watch or type to it; it can't ask questions (AskUserQuestion disabled), and
# tools/overnight-stop.sh ends each session after HST_IDLE (90) idle seconds — typing something resets that.
#
#   tmux new -s hst tools/overnight.sh        # detach: Ctrl-b d · reattach: tmux attach -t hst
#   HST_IDLE=90 HST_PAUSE=60 tools/overnight.sh
#
# Runs with permission prompts bypassed: review the work in the morning (git log), and keep your backup.
set -u
cd "$(dirname "$(realpath "$0")")/.."
log=context/notes/overnight.log
mkdir -p "$(dirname "$log")"
pause=${HST_PAUSE:-60}
run=0
# the virtual pad lives for the whole night (PCSX2 picks it up as SDL-0 when no real pad is connected)
pgrep -f 'vpad.py serve' >/dev/null || { tools/vpad.py serve >>"$log" 2>&1 & vpad=$!; }
trap '[ -n "${vpad:-}" ] && kill "$vpad" 2>/dev/null' EXIT
while grep -q '^- \[ \]' PLAN.md; do
  run=$((run + 1))
  echo "=== run $run $(date -Is) — next: $(grep -m1 '^- \[ \]' PLAN.md | cut -c1-100)" | tee -a "$log"
  claude "continue" --permission-mode bypassPermissions --disallowedTools AskUserQuestion \
    --append-system-prompt "Unattended run: nobody will answer questions. Decide yourself, or follow AGENT.md 'If you get stuck'." \
    --settings "{\"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"$PWD/tools/overnight-stop.sh\"}]}],\"StopFailure\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"$PWD/tools/overnight-stop.sh\"}]}]}}"
  echo "=== run $run exited $? at $(date -Is)" | tee -a "$log"
  sleep "$pause"
done
echo "=== all PLAN tasks checked off at $(date -Is)" | tee -a "$log"
