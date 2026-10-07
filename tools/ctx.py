#!/usr/bin/env python3
"""Read one part of PLAN.md (or another markdown file) by name instead of the whole file.

    tools/ctx.py                 index: every heading and prompt id, one line each
    tools/ctx.py next            the first open prompt, with its sub-items
    tools/ctx.py <ID>            a task's full text, plan/<ID>.md (N1e, F0)
    tools/ctx.py <name>          a heading's section (case-insensitive substring) or an index line by id
    tools/ctx.py -f FILE <name>  same, on another markdown file
"""
import os, re, sys, time

TTL = 5.0  # seconds a parsed file is trusted before its mtime is checked again
_cache = {}  # path -> (checked_at, mtime, lines)
ITEM = re.compile(r'^(\s*)- \[(.)\] \*\*([^\s*]+)')


def lines(path):
    # ponytail: whole-file re-read on change; per-section diffing only if files grow to MBs
    now = time.monotonic()
    hit = _cache.get(path)
    if hit and now - hit[0] < TTL:
        return hit[2]
    mtime = os.stat(path).st_mtime_ns
    if hit and hit[1] == mtime:
        _cache[path] = (now, mtime, hit[2])
        return hit[2]
    with open(path, encoding='utf-8') as f:
        ls = f.read().splitlines()
    _cache[path] = (now, mtime, ls)
    return ls


def _level(line):
    m = re.match(r'(#+) ', line)
    return len(m.group(1)) if m else 0


def _item_block(ls, i):
    indent = len(ITEM.match(ls[i]).group(1))
    j = i + 1
    while j < len(ls) and ls[j].strip() and (len(ls[j]) - len(ls[j].lstrip())) > indent:
        j += 1
    return ls[i:j]


def index(path):
    out = []
    for line in lines(path):
        if _level(line):
            out.append(line)
        elif m := ITEM.match(line):
            out.append(f'{m.group(1)}[{m.group(2)}] {m.group(3)}')
    return out


def section(path, name):
    ls = lines(path)
    if name == 'next':
        for i, line in enumerate(ls):
            m = ITEM.match(line)
            if m and m.group(2) == ' ' and not m.group(1):
                return _item_block(ls, i)
        return []
    for i, line in enumerate(ls):
        m = ITEM.match(line)
        if m and m.group(3).rstrip('.') == name:
            return _item_block(ls, i)
    for i, line in enumerate(ls):
        lv = _level(line)
        if lv and name.lower() in line.lower():
            j = i + 1
            while j < len(ls) and not (0 < _level(ls[j]) <= lv):
                j += 1
            return ls[i:j]
    return []


if __name__ == '__main__':
    args = sys.argv[1:]
    path = os.path.join(os.path.dirname(__file__), '..', 'PLAN.md')
    if args[:1] == ['-f']:
        path, args = args[1], args[2:]
    task = os.path.join(os.path.dirname(__file__), '..', 'plan', f'{args[0]}.md') if args else ''
    if path.endswith('PLAN.md') and os.path.exists(task):
        out = lines(task)
    else:
        out = section(path, args[0]) if args else index(path)
    if not out:
        sys.exit(f'ctx: no section or prompt named {args[0]!r}; run tools/ctx.py for the index')
    print('\n'.join(out))
