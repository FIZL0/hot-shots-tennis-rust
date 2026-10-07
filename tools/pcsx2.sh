#!/usr/bin/env bash
# Run a command with PCSX2 to yourself: tools/pcsx2.sh <cmd...>. Waits while another agent holds it. Use it for
# anything that drives the real game in more than one step (load state + vpad input + record + screenshot),
# e.g. tools/pcsx2.sh bash -c 'tools/vpad.py ... & tools/record_p2m2.py ...'. Single pine.py tools lock themselves.
lock=${XDG_RUNTIME_DIR:-/tmp}/hst-pcsx2.lock
flock -n -o "$lock" true 2>/dev/null || echo "waiting for PCSX2 (another agent is using it)" >&2
exec flock -o "$lock" env HST_PCSX2_LOCKED=1 "$@"
