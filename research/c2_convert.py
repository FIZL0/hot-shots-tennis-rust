#!/usr/bin/env python3
"""C2 helper: rewrite `ps2::{add,sub,mul,div,madd,msub,sqrt,lerp}(..)` calls in one sim module, in place, as
`ps2::F` operator expressions with the same operation tree (`add(add(a, b), c)` → `F(a) + b + c`, each top-level
call ending in `.0`). Afterwards, by hand: the imports, the deny attribute, and whatever clippy's
`float_arithmetic` still flags (plain `f32` ops and unary minus on `f32`). Usage: `research/c2_convert.py FILE`.
Prove it with the hash trace (plan/C2.md)."""
import re, sys

OPS = {'add': '+', 'sub': '-', 'mul': '*', 'div': '/'}
METH = {'madd', 'msub', 'lerp'}
ALL = set(OPS) | METH | {'sqrt'}
PREC = {'+': 1, '-': 1, '*': 2, '/': 2}
NAME = re.compile(r'(?<![\w.:])(?:(?:crate::)?ps2::)?(add|sub|mul|div|madd|msub|sqrt|lerp)\(')


def match_paren(s, i):
    """s[i] == '(' → index of matching ')'."""
    d = 0
    for j in range(i, len(s)):
        c = s[j]
        if c in '([{':
            d += 1
        elif c in ')]}':
            d -= 1
            if d == 0:
                return j
    raise ValueError(s[i:i + 80])


def split_args(s):
    out, d, cur = [], 0, ''
    for c in s:
        if c in '([{':
            d += 1
        elif c in ')]}':
            d -= 1
        if c == ',' and d == 0:
            out.append(cur.strip())
            cur = ''
        else:
            cur += c
    if cur.strip():
        out.append(cur.strip())
    return out


SIMPLE = re.compile(r'^-?[\w.:]+(\[[^\]]*\]|\.\w+|\.\w+\([^()]*\))*$|^-?[\w.:]+\([^()]*\)(\.\w+\(\))*$')


def parse(e):
    """An argument expression: ('op', name, [args]) if it is exactly one op call, else ('leaf', text)."""
    e = e.strip()
    m = NAME.match(e)
    if m and m.start() == 0:
        o = m.end() - 1
        c = match_paren(e, o)
        if c == len(e) - 1:
            return ('op', m.group(1), [parse(a) for a in split_args(e[o + 1:c])])
    return ('leaf', text(e))


def is_float_lit(t):
    return re.fullmatch(r'-?\d+(\.\d+)?(e-?\d+)?', t) is not None


def leaf_atom(t):
    """Leaf as an operand: parenthesised if it isn't atomic."""
    if SIMPLE.match(t) or re.fullmatch(r'-?\d+(\.\d+)?(e-?\d+)?', t):
        return t
    return '(' + t + ')'


def as_f(t):
    """Leaf wrapped to F (also turning a unary minus on a variable into neg.s on F)."""
    if t.startswith('-') and not is_float_lit(t):
        return '-F(' + t[1:] + ')'
    return 'F(' + t + ')'


def operand(t):
    # an f32 leaf used next to an F: a unary minus on a variable is plain f32 negation → move it onto F
    if t.startswith('-') and not is_float_lit(t):
        return '-F(' + t[1:] + ')'
    return leaf_atom(t)


def tr(n):
    """→ (code, prec, is_f): prec 3 = atom/method call."""
    if n[0] == 'leaf':
        return (n[1], 3, False)
    _, op, args = n
    if op in OPS:
        a, b = (tr(x) for x in args)
        sym = OPS[op]
        p = PREC[sym]
        if not a[2] and not b[2]:
            a = (as_f(a[0]), 3, True)
        la = a[0] if a[1] >= p else '(' + a[0] + ')'
        if a[1] == 3 and not a[2]:
            la = operand(a[0])
        rb = b[0] if b[1] > p else '(' + b[0] + ')'
        if b[1] == 3 and not b[2]:
            rb = operand(b[0])
        return (f'{la} {sym} {rb}', p, True)
    recv = tr(args[0])
    if recv[2]:
        r = recv[0] if recv[1] == 3 else '(' + recv[0] + ')'
    else:
        r = as_f(recv[0])
    if op == 'sqrt':
        return (f'{r}.sqrt()', 3, True)
    rest = []
    for x in args[1:]:
        c = tr(x)
        rest.append(c[0] if (c[2] or not c[0].startswith('-') or is_float_lit(c[0])) else operand(c[0]))
    return (f'{r}.{op}({", ".join(rest)})', 3, True)


def text(s):
    """Rewrite every op call in free text, each top-level one ending in `.0` (back to f32)."""
    out, i = '', 0
    while True:
        m = NAME.search(s, i)
        if not m:
            return out + s[i:]
        o = m.end() - 1
        c = match_paren(s, o)
        n = ('op', m.group(1), [parse(a) for a in split_args(s[o + 1:c])])
        code, p, _ = tr(n)
        code = code if p == 3 else '(' + code + ')'
        out += s[i:m.start()] + code + '.0'
        i = c + 1


if __name__ == '__main__':
    p = sys.argv[1]
    src = open(p).read()
    out = text(src).replace('F(0.0) + ', 'F::ZERO + ')
    # drop the parentheses round a plain indexed operand (`a - (b[k])`); brackets may nest once, never span
    out = re.sub(r'([-+*/] )\(([a-z_.0-9]+(\[[^]()[ ]*\]|\[[a-z_.0-9]+\[[^]()[ ]*\]\])+)\)', r'\1\2', out)
    open(p, 'w').write(out)
