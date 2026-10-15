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
# pgrep/pkill -f -> tools/pgrep, which skips the caller's own shell (pkill -f pat killed it, pgrep -f pat always found itself)
ROOT = __file__.rsplit('/tools/', 1)[0]
# test runs: the 120 s default Bash timeout moved check.sh/done.sh to the background, and agents then polled them with
# sleep/pgrep loops for up to 10 min (/home/ryha/repos/opentennis/tools/pgrep -f also matched the other slots' runs). Run them in the foreground with a
# long timeout (BASH_MAX_TIMEOUT_MS in .claude/settings.json allows it) and refuse the polling
TESTS = re.compile(r'tools/(check|done)\.sh|\bcargo (test|build|nextest)\b')
if t.get('tool_name') == 'Bash' and (re.search(r'/tasks/\w+\.output', cmd) or TESTS.search(cmd)) and re.search(
        r'\bsleep\s+([3-9]\d|\d{3,})|\b(while|until)\b[^;]*\bpgrep\b|tail -f --pid', cmd):
    sys.exit(print('Don\'t poll a background task or test run: end your turn now (say what you are waiting for); its '
                   'completion notification wakes you and the runner keeps the session alive meanwhile. Run '
                   'tools/check.sh / tools/done.sh in the foreground: they get a 60 min timeout.', file=sys.stderr) or 2)
# main checkout: several sessions edit it at once, so sweeping commits (add -A/./-u, commit -a) take the others' edits
# (worktrees -- runner slots -- are exempt: their .git dir differs from the common one)
SWEEP = re.compile(r'\bgit\b[^;&|]*\badd\b[^;&|]*\s(-A|--all|-u|--update|\.|:/)(?=\s|$|[;&|)])'
                   r'|\bgit\b[^;&|]*\bcommit\b[^;&|]*\s(-[a-zA-Z]*a[a-zA-Z]*|--all)(?=\s|$|[;&|)])')
if t.get('tool_name') == 'Bash' and SWEEP.search(cmd):
    import subprocess
    d = [subprocess.run(['git', 'rev-parse', '--path-format=absolute', f], cwd=t.get('cwd') or ROOT, capture_output=True,
                        text=True).stdout.strip() for f in ('--git-dir', '--git-common-dir')]
    if d[0] and d[0] == d[1]:
        sys.exit(print('Main checkout is shared with other sessions: stage and commit only your own paths '
                       '(git add <paths>; git commit -m ... -- <paths>), never add -A/./-u or commit -a. '
                       'Bigger work: git worktree add ../opentennis-<slug>.', file=sys.stderr) or 2)
upd = {}
if t.get('tool_name') == 'Bash' and TESTS.search(cmd) and not i.get('run_in_background'):
    upd['timeout'] = 3600000
fixed = re.sub(r'(?<![\w/.-])(pgrep|pkill)(?=(\s+-[\w-]+)*\s+-\w*f)', rf'{ROOT}/tools/\1', cmd)  # -f: matches command lines, so its own too
if t.get('tool_name') == 'Bash' and fixed != cmd:
    upd['command'] = fixed
if upd:
    print(json.dumps({'hookSpecificOutput': {'hookEventName': 'PreToolUse', 'permissionDecision': 'allow',
                                             'updatedInput': {**i, **upd}}}))
