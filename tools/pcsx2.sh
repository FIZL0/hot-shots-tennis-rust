#!/usr/bin/env bash
# Run a command with PCSX2 to yourself: tools/pcsx2.sh <cmd...>. Waits while another agent holds it. Use it for
# anything that drives the real game in more than one step (load state + vpad input + record + screenshot),
# e.g. tools/pcsx2.sh bash -c 'tools/vpad.py ... & tools/record_p2m2.py ...'. Single pine.py tools lock themselves.
lock=${XDG_RUNTIME_DIR:-/tmp}/hst-pcsx2${HST_PCSX2:-}.lock  # HST_PCSX2=N: copy N (tools/pcsx2-hst.sh)
flock -n -o "$lock" true 2>/dev/null || echo "waiting for PCSX2 (another agent is using it)" >&2
# HST_PCSX2_MAX_HOLD (seconds, set by tools/parallel.py): stop a command that holds PCSX2 longer, so a hung
# recorder can't starve the other agents.
flock -o "$lock" ${HST_PCSX2_MAX_HOLD:+timeout -k 10 "$HST_PCSX2_MAX_HOLD"} env HST_PCSX2_LOCKED=1 "$@"; s=$?
[ "$s" = 124 ] && echo "pcsx2.sh: stopped after holding PCSX2 for ${HST_PCSX2_MAX_HOLD}s (the parallel-run cap); split the job into shorter runs" >&2
exit $s
