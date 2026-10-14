#!/usr/bin/env bash
# Add a plan/TODO.md task from a plain sentence, cheaply: one `claude -p` with ~2k tokens of context (code map, source
# file names, two example lines; no tools, no CLAUDE.md) turns it into a task line with its files, then `ctx.py add`
# puts it on top as the next free B id and commits plan/TODO.md alone. Safe while a runner works: no merge in
# progress → its own commit; otherwise it stays uncommitted and the runner stashes it around its merges.
#
#   tools/todo.sh the umpire label should just say the name     (or no args: it asks, so quotes/apostrophes are fine)
#   HST_TODO_RAW=1 tools/todo.sh …                             skip the model, add the sentence as written
set -u
root=$(cd "$(dirname "$(realpath "$0")")/.." && pwd)
cd "$root"
text="$*"
[ -z "$text" ] && read -e -r -p 'todo: ' text
[ -z "$text" ] && exit 1
day=$(date +%F)
line=''
if [ "${HST_TODO_RAW:-0}" != 1 ]; then
  ctx="Code map:
$(awk '/^## Code map/{f=1} /^## Needs the human/{f=0} f' PLAN.md)

Source files:
$(git ls-files 'crates/*.rs' 'web/*' | grep -v /tests/)

Example task lines:
$(grep -m2 '^- \[ \]' plan/TODO.md | sed -E 's/^- \[ \] \*\*[^*]+\*\* //')"
  # run outside the repo so no CLAUDE.md/memory loads; no tools, short system prompt
  line=$(cd "$(mktemp -d)" && printf '%s' "$ctx" | timeout 120 claude -p --model sonnet --tools '' \
    --setting-sources '' --strict-mcp-config --no-session-persistence \
    --system-prompt "You turn the user's note into one task line for a Rust/Bevy game's TODO list, in the examples' style: a short subject, a colon, what's wrong and what it should do (clear, specific, the user's meaning only, invent nothing), then ' (user $day)', then ' → ' and the 1–3 most likely files from the list, paths shortened as in the examples, joined by ' · '. Reply with that line only." \
    "Note: $text" 2>/dev/null | tr '\n' ' ' | sed -E 's/^[-*` ]+//; s/ +$//')
fi
[ -z "$line" ] && line="$text (user $day)"
out=$(tools/ctx.py add B "$line") || exit 1
id=${out#added }
[ -e .git/MERGE_HEAD ] || git commit -q -m "TODO: $id ${text:0:60}" -- plan/TODO.md 2>/dev/null || echo "(left uncommitted)"
echo "$id: $line"
