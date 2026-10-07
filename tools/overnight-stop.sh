#!/usr/bin/env bash
# Stop/StopFailure hook for tools/overnight.sh: once Claude has been idle for HST_IDLE seconds (no background
# tasks, no new user/tool-result entries in the transcript — i.e. you didn't type anything), end the session so the loop starts the next run.
in=$(cat)
[ "$(jq '.background_tasks // [] | length' <<<"$in")" = 0 ] || exit 0
t=$(jq -r .transcript_path <<<"$in")
n() { grep -c '"type":"user"' "$t"; }  # ponytail: transcript size isn't stable after Stop, this count is
s=$(n)
p=$PPID
# a usage limit isn't the end: Claude Code shows "Usage limit reached · continuing automatically at <reset>" and
# resumes the session itself then, so leave it waiting
# (a notice with no tool use after it; a resumed session uses tools, so an old notice doesn't count)
waiting() {
  jq -se '(map(.type == "system" and (.content | tostring | test("continuing automatically"))) | rindex(true)) as $i
    | $i != null and (.[$i:] | all(.type != "assistant" or (.message.content | tostring | test("\"tool_use\"") | not)))' "$t" >/dev/null
}
(sleep "${HST_IDLE:-90}"; [ "$(n)" = "$s" ] && ! waiting && kill "$p") >/dev/null 2>&1 &
