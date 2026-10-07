#!/usr/bin/env python3
"""PreToolUse hook: stop whole-file reads of the 16 MB decompile dumps (context/decomp/*.c) before they flood
the context. Exit 2 = block, stderr goes back to the agent."""
import json, re, sys

t = json.load(sys.stdin)
i = t.get('tool_input', {})
DUMP = re.compile(r'context/decomp/[^ ]*\.c\b|hst_(game|menu|movie)\.c\b')
HOW = ('16 MB decompile dump: use research/fn.sh <addr> (one function), research/xref.py <addr> '
       '(callers/callees, --similar), or Read with offset+limit, or grep -m/-c/| head.')
if t.get('tool_name') == 'Read' and DUMP.search(i.get('file_path', '')) and not i.get('limit'):
    sys.exit(print(HOW, file=sys.stderr) or 2)
cmd = i.get('command', '')
# ponytail: substring heuristic on the shell command; a shell parser if it blocks real work or misses a dump
if t.get('tool_name') == 'Bash' and DUMP.search(cmd) and not re.search(
        r'fn\.sh|xref\.py|\|\s*(head|tail|wc|sed -n)|\bgrep\b[^|]*\s-[a-zA-Z]*[mcl]|\bwc\b|\bsed -n', cmd):
    sys.exit(print(HOW, file=sys.stderr) or 2)
