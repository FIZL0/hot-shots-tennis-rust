#!/usr/bin/env python3
"""M97 dev stats: the user's prompts, tokens used and commits building the remaster, for the website/donate screen.

    tools/dev_stats.py        # scan transcripts, update research/dev-stats.json (ledger) and web/stats.json (totals)

Reads the Claude Code transcripts of every opentennis checkout and slot (~/.claude/projects/*opentennis*/, plus the
pre-rename HST-Remastered ones), subagents included. Numbers only, never transcript text. The ledger keeps each
transcript's counts by file, so a transcript Claude Code later deletes still counts: run it often (release.sh,
done.sh) and keep `cleanupPeriodDays` high in ~/.claude/settings.json.

Prompt = a user message typed by the user: not a tool result, slash command, task notification, compaction summary,
interrupt marker, or the unattended runners' (single.sh/parallel.py) and `claude -p` (todo.sh) launch prompts.
Tokens = API usage of every assistant message (input, output, cache read, cache write), once per message id.
"""
import json, os, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'research/dev-stats.json'
OUT = ROOT / 'web/stats.json'
PROJECTS = Path.home() / '.claude/projects'
NOT_TYPED = ('<command-', '<local-command', '<task-notification', 'This session is being continued',
             '[Request interrupted', 'Unattended run:', 'Parallel unattended run:', 'Caveat:')
KEYS = ('input_tokens', 'output_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens')


def scan(f, seen):
    """Counts for one transcript; `seen` (message ids/uuids already counted in an earlier file) stops a resumed or
    forked session counting its copied history twice."""
    # ponytail: dedupe only against transcripts still on disk; a copy whose original was deleted counts again
    n = dict.fromkeys(('prompts',) + KEYS, 0)
    first = last = None
    for line in open(f, errors='replace'):
        try:
            d = json.loads(line)
        except ValueError:
            continue
        t, ts = d.get('type'), d.get('timestamp')
        if t == 'assistant' and (u := d.get('message', {}).get('usage')):
            mid = d['message'].get('id') or d.get('uuid')
            if mid in seen:
                continue
            seen.add(mid)
            for k in KEYS:
                n[k] += u.get(k) or 0
        elif t == 'user' and not d.get('isMeta') and not d.get('isSidechain') and d.get('entrypoint') != 'sdk-cli':
            m = d.get('message', {}).get('content')
            if not isinstance(m, str):
                if not isinstance(m, list) or any(x.get('type') == 'tool_result' for x in m):
                    continue
                m = ' '.join(x.get('text', '') for x in m if x.get('type') == 'text')
            if not m.strip() or m.lstrip().startswith(NOT_TYPED) or d.get('uuid') in seen:
                continue
            seen.add(d.get('uuid'))
            n['prompts'] += 1
        else:
            continue
        if ts:
            first, last = first or ts, ts
    return n | {'first': first, 'last': last}


def main():
    ledger = json.loads(LEDGER.read_text()) if LEDGER.exists() else {}
    files = [f for d in PROJECTS.glob('*') if 'opentennis' in d.name or 'HST-Remastered' in d.name
             for f in d.rglob('*.jsonl')]
    seen = set()
    for f in sorted(files, key=lambda f: f.stat().st_mtime):
        if (n := scan(f, seen))['first']:  # skip transcripts with nothing counted
            ledger[str(f.relative_to(PROJECTS))] = n
    ledger = dict(sorted(ledger.items()))
    LEDGER.write_text(json.dumps(ledger, separators=(',', ':')).replace('},', '},\n') + '\n')

    tot = {k: sum(v[k] for v in ledger.values()) for k in ('prompts',) + KEYS}
    starts = [v['first'] for v in ledger.values() if v['first']]
    commits = subprocess.run(['git', '-C', ROOT, 'rev-list', '--count', 'HEAD'], capture_output=True, text=True)
    stats = {'prompts': tot['prompts'], 'tokens': sum(tot[k] for k in KEYS),
             'input_tokens': tot['input_tokens'], 'output_tokens': tot['output_tokens'],
             'cache_read_tokens': tot['cache_read_input_tokens'], 'cache_write_tokens': tot['cache_creation_input_tokens'],
             'commits': int(commits.stdout or 0), 'sessions': len(ledger),
             'since': min(starts)[:10] if starts else None, 'updated': max(v['last'] or '' for v in ledger.values())[:10]}
    OUT.write_text(json.dumps(stats, indent=1) + '\n')
    print(json.dumps(stats))


if __name__ == '__main__':
    sys.exit(main())
