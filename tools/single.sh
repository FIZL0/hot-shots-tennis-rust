#!/usr/bin/env bash
# Unattended runner: keeps starting `claude "continue"` (= do the next open task in plan/TODO.md) until no
# unchecked prompts remain. Any exit — finished prompt, error, usage limit — just waits and starts again.
# It's the normal TUI, so attach to watch or type to it; it can't ask questions (AskUserQuestion disabled), and
# tools/agent-stop.sh ends each session after HST_IDLE (90) idle seconds — typing something resets that.
#
#   tmux new -s hst tools/single.sh               # detach: Ctrl-b d · reattach: tmux attach -t hst
#   HST_IDLE=90 HST_PAUSE=60 tools/single.sh
#
# Runs with permission prompts bypassed: review the work in the morning (git log), and keep your backup.
# Like the parallel runner's agents it gets its own PCSX2 copy (HST_PCSX2=6, ../<repo>-slots/pcsx2/s6; parallel runs use 1..N), so your own PCSX2 stays yours.
set -u
cd "$(dirname "$(realpath "$0")")/.."
pgrep -f '^python3 .*([o]vernight-parallel|tools/[p]arallel)\.py' >/dev/null && { echo "a parallel run is going; wait for it to end"; exit 1; }
export HST_PCSX2=6 HST_PCSX2_MAX_HOLD=${HST_PCSX2_MAX_HOLD:-1200}
cfg=../$(basename "$PWD")-slots/pcsx2/s6/PCSX2
[ -d "$cfg" ] || { echo "no PCSX2 copy at $cfg (needs its own PINESlot 28017)"; exit 1; }
# the copy's save states start as the user's (their [Folders] SaveStates, relative to ~/.config/PCSX2)
ss=$(sed -n 's/^SaveStates = //p' ~/.config/PCSX2/inis/PCSX2.ini); [[ $ss = /* ]] || ss=~/.config/PCSX2/${ss:-sstates}
cp -n "$ss"/SCUS-97610* "$cfg/sstates/" 2>/dev/null
prompt="Unattended run: nobody will answer questions. Do the next open task in plan/TODO.md — \`tools/ctx.py next\` — \
following PLAN.md's rules and AGENT.md; commit on main. You have your own PCSX2 (HST_PCSX2=6 is set: tools/pcsx2-hst.sh \
starts copy 6, pine.py and tools/vpad.py talk to it; its save states are a copy, so scratch saves to 8/9 are yours). Run \
anything that drives the game in several steps as one command under \`tools/pcsx2.sh <cmd>\`; each is cut off after \
$((HST_PCSX2_MAX_HOLD / 60)) min, so keep runs short (record less, split it). Save slot 5 is the only bot-only game; 3 and \
4 have P1 human and sit waiting for input unless you drive it with tools/vpad.py in the same tools/pcsx2.sh call. Close \
it with \`tools/pcsx2-hst.sh stop\` (never pkill pcsx2-qt). In the task's journal entry, list under 'Not verified / not 1:1' everything you couldn't verify against the original or couldn't match exactly, each with the reason (or 'none'). When done, `tools/ctx.py tick <ID>` and commit; if stuck, \
mark it \`[~]\` per AGENT.md 'If you get stuck', commit, and stop. A usage limit is not stuck: commit what's solid but \
don't mark it \`[~]\` or stop PCSX2 — the session waits for the reset and continues the task."
log=context/notes/overnight.log
mkdir -p "$(dirname "$log")"
pause=${HST_PAUSE:-60}
run=0
trap 'tools/pcsx2-hst.sh stop' EXIT  # copy 6 starts its own virtual pad and takes it down with it
while grep -q '^- \[ \]' plan/TODO.md; do
  run=$((run + 1))
  echo "=== run $run $(date -Is) — next: $(grep -m1 '^- \[ \]' plan/TODO.md | cut -c1-100)" | tee -a "$log"
  id=$(uuidgen)
  claude "$prompt" --session-id "$id" --permission-mode bypassPermissions --disallowedTools AskUserQuestion \
    --settings "{\"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"$PWD/tools/agent-stop.sh\"}]}],\"StopFailure\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"$PWD/tools/agent-stop.sh\"}]}]}}"
  echo "=== run $run exited $? at $(date -Is)" | tee -a "$log"
  tools/pcsx2.sh tools/pcsx2-hst.sh stop  # as the parallel runner: after the session, once nothing holds it
  t=$(find ~/.claude/projects -name "$id.jsonl" 2>/dev/null | head -1)
  reset=$([ -n "$t" ] && grep -o 'limit · resets [0-9:]*[ap]m' "$t" | tail -1 | grep -o '[0-9:]*[ap]m')
  wait=$pause
  if [ -n "$reset" ] && ! grep -q '"type":"tool_use"' "$t"; then
    # the run only hit the usage limit: drop its transcript and sleep until the reset instead of relaunching
    rm -rf "$t" "${t%.jsonl}"
    wait=$(( $(date -d "$reset" +%s) - $(date +%s) ))
    [ "$wait" -lt -600 ] && wait=$((wait + 86400))  # reset is tomorrow
    [ "$wait" -lt 0 ] && wait=0                     # just ticked over
    wait=$((wait + pause))
    echo "=== run $run only hit the limit; transcript discarded, sleeping until $reset" | tee -a "$log"
  fi
  sleep "$wait"
done
echo "=== all PLAN tasks checked off at $(date -Is)" | tee -a "$log"
