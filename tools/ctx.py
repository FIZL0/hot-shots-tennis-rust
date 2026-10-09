#!/usr/bin/env python3
"""Read one part of the plan by name instead of whole files, and tick/block tasks.

    tools/ctx.py                 index of plan/TODO.md: every heading and task id, one line each
    tools/ctx.py next            the first open task, with its sub-items
    tools/ctx.py <ID>            a task's full text: plan/<ID>.md, else its line in TODO.md or DONE.md
    tools/ctx.py <name>          a heading's section (case-insensitive substring) in TODO.md, PLAN.md or DONE.md
    tools/ctx.py -f FILE <name>  same, on another markdown file
    tools/ctx.py tick <ID>       mark done: [x] here and in plan/<ID>.md, move the line to plan/DONE.md
    tools/ctx.py block <ID> WHY  mark [~] with "BLOCKED: WHY" appended
    tools/ctx.py archive         move every [x] line left in TODO.md to DONE.md (after merges, hand edits)
    tools/ctx.py brief <ID>      everything a task starts from: its line, plan/<ID>.md, its journal (newest file in
                                 full), where its files are and their last commits; the runners paste it in the prompt
"""
import os, re, subprocess, sys, time

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


def _write(path, ls):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write('\n'.join(ls).rstrip('\n') + '\n')
    os.replace(tmp, path)
    _cache.pop(path, None)


def _find(ls, tid):
    for i, line in enumerate(ls):
        m = ITEM.match(line)
        if m and not m.group(1) and m.group(3).rstrip('.') == tid:
            return i
    raise SystemExit(f'ctx: no task {tid!r}')


def _mark(ls, i, box):
    ls[i] = re.sub(r'^- \[.\]', f'- [{box}]', ls[i])


def archive(todo, done):
    """Move every top-level [x] task (with its sub-lines) from todo into done, under the same ### heading."""
    ls, dl, sec, moved = list(lines(todo)), list(lines(done)), '', 0
    i = 0
    while i < len(ls):
        if _level(ls[i]):
            sec = ls[i]
        m = ITEM.match(ls[i])
        if not (m and not m.group(1) and m.group(2) == 'x'):
            i += 1
            continue
        block = _item_block(ls, i)
        del ls[i:i + len(block)]
        if i < len(ls) and not ls[i].strip() and i and not ls[i - 1].strip():
            del ls[i]  # don't leave a double blank line
        h = sec or '### (no section)'
        if h not in dl:
            dl += ['', h, '']
        j = dl.index(h) + 1
        while j < len(dl) and not _level(dl[j]):
            j += 1
        while dl[j - 1].strip() == '':
            j -= 1
        dl[j:j] = block
        moved += 1
    if moved:
        _write(done, dl)
        _write(todo, ls)
    return moved


def tick(todo, done, tid, plan_dir=None):
    ls = list(lines(todo))
    _mark(ls, _find(ls, tid), 'x')
    _write(todo, ls)
    own = plan_dir and os.path.join(plan_dir, f'{tid}.md')
    if own and os.path.exists(own):
        _write(own, [re.sub(rf'^(\s*)- \[[ ~]\] \*\*{re.escape(tid)}\b', rf'\1- [x] **{tid}', l) for l in lines(own)])
    return archive(todo, done)


def block(todo, tid, why):
    ls = list(lines(todo))
    i = _find(ls, tid)
    _mark(ls, i, '~')
    end = i + len(_item_block(ls, i)) - 1
    ls[end] += f' BLOCKED: {why}'
    _write(todo, ls)


JOURNAL_MAX = 12000  # chars of the newest journal file put in a brief; the rest is a Read away


def brief(root, tid):
    """The task's starting context in one go, so a session doesn't spend its first turns gathering it."""
    plan_dir = os.path.join(root, 'plan')
    paths = [os.path.join(plan_dir, 'TODO.md'), os.path.join(plan_dir, 'DONE.md')]
    line = next((o for p in paths if (o := section(p, tid))), [])
    if not line:
        raise SystemExit(f'ctx: no task {tid!r}')
    text = '\n'.join(line)
    out = [f'## Task {tid} (plan/TODO.md)', *line]
    own = os.path.join(plan_dir, f'{tid}.md')
    if os.path.exists(own):
        out += ['', f'## plan/{tid}.md', *lines(own)]
        text += '\n'.join(lines(own))
    jdir = os.path.join(root, 'research/journal')
    named = re.findall(r'\bj: ([\w.-]+)', text)
    own_j = [d for d in sorted(os.listdir(jdir)) if re.search(rf'-{re.escape(tid.lower())}(-|$)', d.lower())]
    for j in dict.fromkeys(named + own_j):
        d = os.path.join(root, 'research/journal', j)
        if not os.path.isdir(d):
            continue
        files = sorted(f for f in os.listdir(d) if f.endswith('.md'))
        out += ['', f'## Journal research/journal/{j}/: {" · ".join(files)}']
        if files:
            body = open(os.path.join(d, files[-1]), encoding='utf-8').read()
            cut = len(body) > JOURNAL_MAX
            out += [f'### {files[-1]}' + (f' (first {JOURNAL_MAX} chars)' if cut else ''), body[:JOURNAL_MAX].rstrip()]
    names = list(dict.fromkeys(re.findall(r'[\w-]+\.(?:rs|py|sh|md|toml)\b', text.split('| t:')[0])))
    tracked = subprocess.run(['git', 'ls-files'], cwd=root, capture_output=True, text=True).stdout.split()
    where = [(n, [t for t in tracked if t.endswith('/' + n) or t == n]) for n in names]
    where = [(n, ts) for n, ts in where if ts and n != f'{tid}.md']
    if where:
        out += ['', '## Files on the line, and their last commits']
        for n, ts in where:
            if len(ts) > 1:
                out.append(f'- `{n}`: one of ' + ' · '.join(f'`{t}`' for t in ts))
                continue
            for t in ts:
                log = subprocess.run(['git', 'log', '-3', '--format=%h %ad %s', '--date=short', '--', t], cwd=root,
                                     capture_output=True, text=True).stdout.strip().splitlines()
                n_lines = sum(1 for _ in open(os.path.join(root, t), encoding='utf-8', errors='replace'))
                out += [f'- `{t}` ({n_lines} lines)', *[f'  - {l[:120]}' for l in log]]
    return out


if __name__ == '__main__':
    args = sys.argv[1:]
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
    plan_dir = os.path.join(root, 'plan')
    todo, done = os.path.join(plan_dir, 'TODO.md'), os.path.join(plan_dir, 'DONE.md')
    if args[:1] == ['tick'] and len(args) == 2:
        tick(todo, done, args[1], plan_dir)
        sys.exit(print(f'ticked {args[1]}, moved to plan/DONE.md'))
    if args[:1] == ['block'] and len(args) > 2:
        block(todo, args[1], ' '.join(args[2:]))
        sys.exit(print(f'blocked {args[1]}'))
    if args[:1] == ['brief'] and len(args) == 2:
        sys.exit(print('\n'.join(brief(root, args[1]))))
    if args == ['archive']:
        sys.exit(print(f'archived {archive(todo, done)}'))
    paths = [todo, os.path.join(root, 'PLAN.md'), done]
    if args[:1] == ['-f']:
        paths, args = [args[1]], args[2:]
    task = os.path.join(plan_dir, f'{args[0]}.md') if args else ''
    if len(paths) > 1 and os.path.exists(task):
        out = lines(task)
    elif not args:
        out = index(paths[0])
    else:
        out = next((o for p in paths if (o := section(p, args[0]))), [])
    if not out:
        sys.exit(f'ctx: no section or prompt named {args[0]!r}; run tools/ctx.py for the index')
    print('\n'.join(out))
