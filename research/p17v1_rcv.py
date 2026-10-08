# P17v1: write context/p17v/rcv.txt, the shade receiver vertices of cap0.pkl with their inputs, and check the model.
# Inputs: context/p17v/h8/eeMemory.bin (EE RAM of the halt state made by research/p17v1_halt.py: slot 5, court 10,
# the build halted at its first tile's read-back), context/p17v/cap0.pkl (GS dump of the build) and
# context/p17v/hole.txt (the disc's court 10 hole model vertices, f32 bits, "P" starts a packet).
# Model (exact on every receiver triangle vertex): clip = (item rows x VP) . (p, 1), Q = 1/clip.w, X Y = FTOI4(clip.xy Q);
# (s, t, w) = T . (p, 1) with T = light rows x bias (EE madd order 1 0 2 3), S T Q = (s, t, w) Q; VU ops chopped.
import pickle, struct, collections
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
vmadd = lambda c, a, b: chop(F(c) + F(mul(a, b)))
def ee_add(a, b):  # EE FPU: the smaller operand loses the bits shifted out, one guard bit kept
    ba, bb = b32(a), b32(b); ea, eb = (ba >> 23) & 0xff, (bb >> 23) & 0xff
    mk = lambda x, d: x & 0x80000000 if d >= 25 else x & ~((1 << (d - 1)) - 1)
    if ea > eb: b = f32(mk(bb, ea - eb))
    elif eb > ea: a = f32(mk(ba, eb - ea))
    return chop(F(a) + F(b))
emadd = lambda c, a, b: ee_add(c, mul(a, b))
xf = lambda m, v: [vmadd(vmadd(vmadd(mul(m[0][k], v[0]), m[1][k], v[1]), m[2][k], v[2]), m[3][k], v[3]) for k in range(4)]

m = open('context/p17v/h8/eeMemory.bin', 'rb').read()
u = lambda a: struct.unpack_from('<I', m, a)[0]
mat = lambda a: [list(struct.unpack_from('<4f', m, a + 16 * i)) for i in range(4)]
GAME, CAM, RCV = u(0x1ea5d8), 0x1fff3d0, 0x16cf5c0  # shade state, shade camera (stack), the hole receiver model
n, lst = u(GAME + 0x1b840), u(GAME + 0x1b844)
B = mat(GAME + 0x1b7b0)
VP, item = mat(CAM + 0x1e0), mat(RCV + 0xe0)
M = [xf(VP, r) for r in item]
Ls = [mat(u(lst + 4 * c) + 0x10) for c in range(n)]
tex = collections.defaultdict(list)  # shadow texture k (TBP 0x3308 + 0x20 k) -> casters drawing into it
for c in range(n):
    T = [[emadd(emadd(emadd(mul(r[1], B[1][j]), r[0], B[0][j]), r[2], B[2][j]), r[3], B[3][j]) for j in range(4)] for r in Ls[c]]
    tex[(u(u(lst + 4 * c) + 0xd0) - 0x620010) // 0x4c0].append((c, T))
pts = set()
for l in open('context/p17v/hole.txt'):
    s = l.split()
    if s[0] != 'P': pts.add(tuple(f32(int(s[k], 16)) for k in range(3)))
scr = collections.defaultdict(list)
for p in pts:
    cl = xf(M, [*p, 1.0]); q = chop(F(1) / F(cl[3]))
    scr[(int(mul(cl[0], q) * 16), int(mul(cl[1], q) * 16))].append((p, q))
out, res = set(), collections.Counter()
for fr, st, vs in pickle.load(open('context/p17v/cap0.pkl', 'rb')):
    k = ((st['tex0'] & 0x3fff) - 0x3308) // 0x20
    if not 0 <= k < 16 or st['tex0'] == 0 or st['prim'] & 7 == 6: continue  # sprites (Z max, Q 1) aren't the mesh
    for v in vs:
        hit = [(c, p) for p, q in scr.get((v[0], v[1]), []) for c, T in tex[k] if tuple(mul(x, q) for x in xf(T, [*p, 1.0])[:3]) == tuple(v[3:6])]
        res[bool(hit)] += 1
        if hit: out.add((hit[0][0],) + tuple(map(b32, hit[0][1])) + (v[0], v[1]) + tuple(map(b32, v[3:6])))
print('vertices matched / not:', res[True], res[False], 'unique', len(out))
with open('context/p17v/rcv.txt', 'w') as f:
    f.write('# P17v1: the shade receiver vertices of context/p17v/cap0.pkl (court 10 hole, slot 5) with their inputs, as\n'
            '# f32 bits: VP = the shade camera world->screen matrix, ITEM = the receiver model matrix, L c = caster c light\n'
            '# matrix (game RAM, halt state); then c px py pz X Y S T Q (X Y = GS 12.4 integers in hex). research/p17v1_rcv.py\n')
    f.write('VP ' + ' '.join('%08x' % b32(x) for r in VP for x in r) + '\n')
    f.write('ITEM ' + ' '.join('%08x' % b32(x) for r in item for x in r) + '\n')
    for c in range(n): f.write('L %d ' % c + ' '.join('%08x' % b32(x) for r in Ls[c] for x in r) + '\n')
    for o in sorted(out): f.write('%d ' % o[0] + ' '.join('%08x' % x for x in o[1:]) + '\n')
