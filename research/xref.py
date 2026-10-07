#!/usr/bin/env python3
"""Who calls a decompiled function, what it calls, and which functions look like it.

    research/xref.py 350280            callers and callees (address, name, size)
    research/xref.py 350280 --similar  the 10 functions whose decompiled code is shaped most like it;
                                       `ported` = named in a research/journal/*FINAL*.md, so its Rust is a template
    -f FILE                            another dump (default context/decomp/hst_game.c, like fn.sh)
"""
import glob, os, re, sys
from collections import Counter
from math import log, sqrt

ROOT = os.path.join(os.path.dirname(os.path.realpath(__file__)), '..')
HEAD = re.compile(r'^// 00([0-9a-f]{6}) ')
WORD = re.compile(r'[A-Za-z_]\w*|\d\w*|\S')
KEEP = {'if', 'else', 'while', 'do', 'for', 'return', 'switch', 'case', 'break', 'goto', 'float', 'int', 'uint',
        'long', 'char', 'short', 'bool', 'undefined4', 'undefined8', 'sqrtf', 'sinf', 'cosf', 'atan2f'}


def load(path):
    names = {}
    for row in open(path.replace('.c', '.funcs.tsv')):
        a, name, size, _ = row.rstrip('\n').split('\t')
        names[name] = (a[2:], int(size))
    bodies, cur = {}, None
    for line in open(path, encoding='utf-8', errors='replace'):
        if m := HEAD.match(line):
            cur = m.group(1)
            bodies[cur] = []
        elif cur:
            bodies[cur].append(line)
    return names, {a: ''.join(b) for a, b in bodies.items()}


def calls(names, body):
    return {names[w][0] for w in re.findall(r'(\w+)\(', body) if w in names}


def shape(body):
    # ponytail: tf-idf token-trigram cosine over the decompiled C; asm opcode n-grams (Coddog) if this ranks badly
    t = [w if w in KEEP or not (w[0].isalpha() or w[0] in '_0123456789') else
         ('N' if w[0].isdigit() else 'ID') for w in WORD.findall(body.split('{', 1)[-1])]
    return Counter(zip(t, t[1:], t[2:]))


def cos(a, b):
    dot = sum(v * b[k] for k, v in a.items() if k in b)
    return dot / (sqrt(sum(v * v for v in a.values())) * sqrt(sum(v * v for v in b.values())) or 1)


def ported():
    seen = {}
    for f in glob.glob(os.path.join(ROOT, 'research/journal/*/*FINAL*.md')):
        for a in re.findall(r'\b0x([0-9a-f]{6})\b', open(f).read()):
            seen.setdefault(a, os.path.relpath(f, ROOT))
    return seen


if __name__ == '__main__':
    args = sys.argv[1:]
    path = os.path.join(ROOT, 'context/decomp/hst_game.c')
    if '-f' in args:
        i = args.index('-f')
        path = args.pop(i + 1)
        args.pop(i)
    if not args:
        sys.exit(__doc__)
    addr = args[0].lower().removeprefix('0x').removeprefix('00').rjust(6, '0')
    names, bodies = load(path)
    if addr not in bodies:
        sys.exit(f'xref: no function starts at {addr} in {os.path.basename(path)}')
    info = {a: (n, s) for n, (a, s) in names.items()}
    row = lambda a, extra='': f'  {a}  {info.get(a, ("?", 0))[0]:28} {info.get(a, ("?", 0))[1]:6}{extra}'
    if '--similar' in args:
        shapes = {a: shape(b) for a, b in bodies.items()}
        df = Counter(k for s in shapes.values() for k in s)
        idf = lambda s: {k: v * log(len(shapes) / df[k]) for k, v in s.items()}
        done, me = ported(), idf(shapes[addr])
        best = sorted(((cos(me, idf(s)), a) for a, s in shapes.items() if a != addr), reverse=True)[:10]
        print(f'like {addr} (score, address, name, size):')
        for score, a in best:
            print(f'{score:.2f}' + row(a, f'  ported: {done[a]}' if a in done else ''))
    else:
        me = names.get(info.get(addr, ('',))[0], (addr,))[0]
        callers = sorted(a for a, b in bodies.items() if a != addr and me in calls(names, b))
        print(f'{addr} called by:'), [print(row(a)) for a in callers]
        print(f'{addr} calls:'), [print(row(a)) for a in sorted(calls(names, bodies[addr]) - {addr})]
