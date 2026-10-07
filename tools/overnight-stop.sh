#!/usr/bin/env bash
# Stop/StopFailure hook for tools/overnight.sh: once Claude has been idle for HST_IDLE seconds (no background
# tasks, no new user/tool-result entries in the transcript — i.e. you didn't type anything), end the session so the loop starts the next run.
in=$(cat)
[ "$(jq '.background_tasks // [] | length' <<<"$in")" = 0 ] || exit 0
t=$(jq -r .transcript_path <<<"$in")
n() { grep -c '"type":"user"' "$t"; }  # ponytail: transcript size isn't stable after Stop, this count is
s=$(n)
p=$PPID
(sleep "${HST_IDLE:-90}"; [ "$(n)" = "$s" ] && kill "$p") >/dev/null 2>&1 &
