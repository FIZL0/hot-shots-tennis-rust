"""Map the game's TParam.csv parser to per-character record offsets and print every character's value per field.
The parser reads cells with strtok over ',' (empty cells are skipped) and strtok_r over '/' inside a cell.
usage: research/fn.sh 18b5e0 > parser.c; python3 research/p7c_tparam_map.py parser.c [TParam.csv]"""
import re, csv, io, sys

body = open(sys.argv[1]).read()
lines = body.split('\n')
ops = []  # [kind, var, scale]: kind 'c' next ',' cell, 's0' first '/' part of it, 's' next '/' part
for l in lines:
    if 'FUN_00120ac8(0,' in l:
        ops.append(['c', None, None])
    elif 'FUN_00120ae8(uVar2' in l:
        ops.append(['s0', None, None])
    elif 'FUN_00120ae8(0,' in l:
        ops.append(['s', None, None])
    m = re.match(r'\s*(\w+) = FUN_(0011aae8|001b32b0)\(uVar2\);', l)
    if m:
        ops[-1][1] = m.group(1)
        ops[-1][2] = 'int' if m.group(2) == '0011aae8' else 'atof'
    m = re.match(r'\s*(\w+) = (\w+ \+ )?\(float\)iVar1 / (10|100)\.0;', l)
    if m:
        ops[-1][1] = m.group(1) + ('+' if m.group(2) else '')
        ops[-1][2] = '/' + m.group(3)
store = {}
for l in lines:
    m = re.match(r'\s*\*\((?:undefined4|float) \*\)\(&DAT_002f0(\w+) \+ param_1\) = (.+);', l) or \
        re.match(r'\s*\(&DAT_002f0(\w+)\)\[param_1\] = (.+);', l)
    if m:
        store[m.group(2).strip()] = int(m.group(1), 16) - 0x880
store['fStack_48+'] = store['fStack_48']
ops[-1][1] = '(float)iVar1 / 100.0'
ops[-1][2] = '/100'

path = sys.argv[2] if len(sys.argv) > 2 else 'context/p7c/TParam.csv'
rows = list(csv.reader(io.StringIO(open(path, 'rb').read().decode('shift_jis'))))
head = rows[0]
data = [r for r in rows[1:] if r and r[0].isdigit()]


def walk(r):
    toks = [(j, c) for j, c in enumerate(r) if j > 0 and c != '']
    k, out = -1, []
    for kind, var, scale in ops:
        if kind == 'c':
            k += 1
            col, cell = toks[k]
            si, val = 0, cell
        elif kind == 's0':
            sub, si = cell.split('/'), 0
            val = sub[0]
        else:
            si += 1
            val = sub[si] if si < len(sub) else None
        if var in store:
            out.append((store[var], col, si, val, scale))
    return out


for off, col, si, _, scale in walk(data[0]):
    vals = [{w[0]: w[3] for w in walk(r)}[off] for r in data]
    h = head[col].replace('\n', ' ')
    print(f'+0x{0x12d8 + off:04x} col{col:2}/{si} {scale:5} {h[:24]:24} {" ".join(vals)}')
