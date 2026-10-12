# P17v6: the shade receiver cull (which hole triangles each caster's pass draws) run on the halt state's RAM, against
# the triangles cap0.pkl draws per caster. Inputs as research/p17v1_rcv.py (context/p17v/h8/eeMemory.bin, cap0.pkl,
# rcv.txt from p17v1_rcv.py). Model: planes (caster +0x50, two transposed blocks) taken into model space by the node
# matrix (identity for the hole: the per-node transform leaves them, and only so do they reproduce cap0); node, batch and packet boxes tested (VU0: culled when d + |n|·half + n·centre ≤ 0 for any plane); each strip
# vertex closes a triangle that is culled when all three corners are on the outside of one plane, or (when one is
# outside the cap plane) when it faces away from the sun; runs of three culled triangles break the strip.
# Prints per caster: drawn by the game, by the model, missing, extra.  `-w` writes context/p17v/cull.txt.
import pickle, struct, sys, collections
from fractions import Fraction as F
f32 = lambda u: struct.unpack('<f', struct.pack('<I', u & 0xffffffff))[0]
b32 = lambda x: struct.unpack('<I', struct.pack('<f', x))[0]
def chop(fr):
    if fr == 0: return 0.0
    v = f32(b32(float(fr)))
    if abs(F(v)) > abs(fr): v = f32(b32(v) - 1)
    elif abs(F(f32(b32(v) + 1))) <= abs(fr): v = f32(b32(v) + 1)
    return v
mul = lambda a, b: chop(F(a) * F(b))
add = lambda a, b: chop(F(a) + F(b))
madd = lambda c, a, b: add(c, mul(a, b))
def ee_add(a, b):  # EE FPU: the smaller operand loses the bits shifted out, one guard bit kept
    ba, bb = b32(a), b32(b); ea, eb = (ba >> 23) & 0xff, (bb >> 23) & 0xff
    mk = lambda x, d: x & 0x80000000 if d >= 25 else x & ~((1 << (d - 1)) - 1)
    if ea > eb: b = f32(mk(bb, ea - eb))
    elif eb > ea: a = f32(mk(ba, eb - ea))
    return chop(F(a) + F(b))
WHY = []
neg = lambda x: x < 0 or (x == 0 and str(x)[0] == '-')

m = open('context/p17v/h8/eeMemory.bin', 'rb').read()
u = lambda a: struct.unpack_from('<I', m, a & 0x1ffffff)[0]  # pointers may be in the uncached mirrors
fl = lambda a, n=4: list(struct.unpack_from('<%df' % n, m, a & 0x1ffffff))
mat = lambda a: [fl(a + 16 * i) for i in range(4)]
GAME, RCV = u(0x1ea5d8), 0x16cf5c0
n, lst = u(GAME + 0x1b840), u(GAME + 0x1b844)
casters = [u(lst + 4 * c) for c in range(n)]
mi = u(RCV + 0xc); root, model = u(mi + 100), u(mi + 0x54)

I4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
def xplanes(cp, M):
    # each block: row k = P0·Mk.x + P1·Mk.y + P2·Mk.z + P3·Mk.w (VU0 mula/madda chain)
    out = []
    for b in range(2):
        P = mat(cp + 0x50 + 0x40 * b)
        out.append([[madd(madd(madd(mul(P[0][j], M[k][0]), P[1][j], M[k][1]), P[2][j], M[k][2]), P[3][j], M[k][3]) for j in range(4)] for k in range(4)])
    return out

def boxcut(B, c, h):  # nonzero = culled
    for b in range(2):
        R = B[b]; A = [[abs(x) for x in R[k]] for k in range(3)]
        for j in range(4):
            v = R[3][j]
            for (row, s) in ((A[2], h[2]), (A[1], h[1]), (A[0], h[0]), (R[2], c[2]), (R[1], c[1]), (R[0], c[0])):
                v = madd(v, row[j], s)
            if v == 0 or neg(v): return True
    return False

def signs(B, p):
    out = []
    for b in range(2):
        R = B[b]
        for j in range(4):
            out.append(neg(madd(madd(madd(R[3][j], R[2][j], p[2]), R[1][j], p[1]), R[0][j], p[0])))
    return out

def tri_cut(B, v1, v2, v3, w, sticky):
    s = [signs(B, v) for v in (v1, v2, v3)]
    if any(s[0][k] and s[1][k] and s[2][k] for k in range(8)): return 1
    if not (s[0][0] or s[1][0] or s[2][0]): return 0
    if sticky: return 0
    a = [mul(add(v2[k], -v1[k]), -w) for k in range(3)]  # (v2 − v1)·(0 − w)
    b4 = [add(v3[k], -v1[k]) for k in range(3)]
    N = [add(mul(a[1], b4[2]), -mul(b4[1], a[2])), add(mul(a[2], b4[0]), -mul(b4[2], a[0])), add(mul(a[0], b4[1]), -mul(b4[0], a[1]))]
    R = B[1]
    d = madd(madd(mul(R[2][3], N[2]), R[1][3], N[1]), R[0][3], N[0])
    return 1 if (neg(d) and d != 0) else 0

def strip(B, pos, flags, uvw, sticky):
    out, h = [], 0
    for i, p in enumerate(pos):
        h = (h & 3) << 1
        r = 1 if flags[i] & 0x8000 else tri_cut(B, pos[i - 2], pos[i - 1], p, uvw[i], sticky)
        h |= r
        if h == 7:
            h = 3
            out[-2] = (out[-1][0], 0x8000); out[-1] = (p, 0x8000)
        else: out.append((p, flags[i] & 0x8000))
    while h & 1: out.pop(); h >>= 1
    return out

def walk(cp, nd, sticky, tris):
    P = u(nd + 0x108); hd = u(P)
    lo, hi = fl(hd), fl(hd + 16)
    B = xplanes(cp, I4)
    if u(cp + 0x11c) == 0: return
    if any(abs(x) == float('inf') for x in lo + hi): return False  # empty node (inf box)
    if boxcut(B, [mul(ee_add(lo[k], hi[k]), 0.5) for k in range(3)], [mul(ee_add(hi[k], -lo[k]), 0.5) for k in range(3)]): WHY.append(('node', u(nd + 0x10c))); return False
    ent = u(u(model + 0x30)) + u(nd + 0x10c) * 0xc
    for e in range(u(ent + 4)):
        mb, bb = m[(u(ent + 8) & 0x1ffffff) + 4 * e], m[(u(ent + 8) & 0x1ffffff) + 4 * e + 1]
        mt = u(model + 0x40) + mb * 0x60
        fv = u(u(mt + 0x10) + 0x10)
        if not (u(GAME + 0x1b92c) & 0xff or (fv & 6 == 0 and fl(u(u(mt + 0x58)) + 0xc, 1)[0] == 128.0)): continue
        br = u(u(u(u(u(model + 0x1c) + 8) + mb * 4) + 8) + bb * 4)
        if not m[(u(br + 4) & 0x1ffffff) + 0x30] & 8: continue
        blo, bhi = fl(br + 0x20), fl(br + 0x30)
        if boxcut(B, [mul(ee_add(blo[k], bhi[k]), 0.5) for k in range(3)], [mul(ee_add(bhi[k], -blo[k]), 0.5) for k in range(3)]): WHY.append(('batch', u(nd + 0x10c), mb, bb, blo, bhi)); continue
        for q in range(u(u(br + 4) + 0x1c)):
            pr = u(u(br + 0x14) + 4 * q); pf = u(pr + 4); base = u(pr + 0xc); b0 = m[u(pr + 0x10) & 0x1ffffff]
            hf = fl(pf, 8)
            if boxcut(B, fl(pf + 0x20, 3), [mul(ee_add(hf[4 + k], -hf[k]), 0.5) for k in range(3)]): continue
            p0 = base + u(pf + 0x44) * 16 + b0 * 16; cnt = u(pf + 0x3c)
            pos = [tuple(fl(p0 + 16 * i, 3)) for i in range(cnt)]
            fa = base + u(pf + 0x48) * 16 + b0 * 8
            flags = [struct.unpack_from("<H", m, (fa & 0x1ffffff) + 8 * i + 6)[0] for i in range(cnt)]
            uvw = [fl(base + u(pf + 0x50) * 16 + 16 * i + 12, 1)[0] for i in range(cnt)]
            o = strip(xplanes(cp, I4), pos, flags, uvw, sticky)  # the strip test uses the caster's own planes
            for i in range(2, len(o)):
                if not o[i][1]: tris.append(frozenset((q[0], q[2]) for q in (o[i - 2][0], o[i - 1][0], o[i][0])))  # y is flattened away
    return True

def rec(cp, nd, sticky, tris):
    if walk(cp, nd, sticky, tris):
        P = u(nd + 0x108)
        for k in range(u(P + 0x1c)): rec(cp, u(u(nd + 0x104) + 4 * k), sticky, tris)

# the game's: cap0 triangles → caster and model positions through rcv.txt's (X Y S T Q) → (c, p)
key = {}
for l in open('context/p17v/rcv.txt'):
    s = l.split()
    if s[0][0] in '#VIL': continue
    key[tuple(int(x, 16) for x in s[4:9])] = (int(s[0]), tuple(f32(int(x, 16)) for x in s[1:4]))
game = collections.defaultdict(set); unk = 0
for fr, st, vs in pickle.load(open('context/p17v/cap0.pkl', 'rb')):
    k = ((st['tex0'] & 0x3fff) - 0x3308) // 0x20
    if not 0 <= k < 16 or st['tex0'] == 0 or st['prim'] & 7 == 6: continue
    hit = [key.get((v[0], v[1]) + tuple(map(b32, v[3:6]))) for v in vs]
    if None in hit or len({h[0] for h in hit}) != 1: unk += 1; continue
    game[hit[0][0]].add(frozenset((h[1][0], h[1][2]) for h in hit))
print('cap triangles not mapped:', unk)
for sticky in (() if "-d" in sys.argv else (False, True)):
    bad = 0; res = {}
    for c, cp in enumerate(casters):
        tris = []; rec(cp, root, sticky, tris)
        mine = {t for t in tris if len(t) == 3}
        g = {t for t in game.get(c, set()) if len(t) == 3}
        res[c] = mine
        print('sticky' if sticky else 'flagS', c, 'game', len(g), 'model', len(mine), 'missing', len(g - mine), 'extra', len(mine - g))
        bad += len(g ^ mine)
    print('total mismatches', bad)
if '-w' in sys.argv:  # per caster its plane blocks as the game left them (P) and the game's triangles, x z bits (T)
    with open('context/p17v/cull.txt', 'w') as o:
        o.write('# P caster 32 plane words (caster +0x50); T caster x z x z x z (cap0 receiver triangles)\n')
        for c, cp in enumerate(casters):
            o.write('P %d %s\n' % (c, ' '.join('%08x' % u(cp + 0x50 + 4 * i) for i in range(32))))
            for t in sorted(game.get(c, set()), key=sorted):
                if len(t) == 3: o.write('T %d %s\n' % (c, ' '.join('%08x %08x' % (b32(x), b32(z)) for x, z in sorted(t))))
if len(sys.argv) > 1 and sys.argv[1] == '-d':
    c = int(sys.argv[2]); tris = []; rec(casters[c], root, True, tris)
    mine = {t for t in tris if len(t) == 3}; g = {t for t in game.get(c, set()) if len(t) == 3}
    for t in sorted(g - mine, key=sorted): print('missing', sorted(t))
    for t in sorted(mine - g, key=sorted): print('extra  ', sorted(t))
    for w in WHY: print(w)
