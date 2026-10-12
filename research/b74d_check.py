"""B74d: checks the net-hiding camera test against the running game. Loads a slot, then on every vsync reads the
net's 'not drawn' byte and the court view (eye, forward, fov) and evaluates the port's rule; prints the frames where
the rule and the game disagree, plus every flip. The distance limit is 2.0 m while the game's "post-point cut-away
on" byte (match state + 0x374) is set, else 0.8 m. Usage: b74d_check.py slot first_vsync last_vsync net_obj_hex"""
import sys, struct, time, math
sys.path.insert(0, 'tools')
from pine import Pine, VSYNC
slot, v0, v1, obj = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4], 16)
p = Pine(); p.load_state(slot); time.sleep(3)
f = lambda a: struct.unpack('<f', struct.pack('<I', p._u32(a)))[0]
def hide(e, d, fov, lim):
    k = max(1.0, math.tan(math.radians(20)) / math.tan(math.radians(fov) / 2))
    if not abs(e[0]) < 7.0 * k: return False
    cos = abs(d[2]) / math.hypot(d[0], d[2])
    if not abs(e[2]) / cos < lim * k: return False
    top = math.atan((0.91 + e[1]) / abs(e[2]))
    low = math.atan2(-d[1], math.hypot(d[0], d[2])) - math.atan(math.tan(math.radians(fov) / 2) * 0.7) * 0.8
    return low < top
last = prev = None; bad = n = 0
while True:
    vs = p._u32(VSYNC)
    if vs == last: continue
    last = vs
    if vs < v0: continue
    if vs > v1: break
    b = p._u32(obj + 4) >> 8 & 0xff
    e = [f(0x1e7d20 + 4 * i) for i in range(3)]; d = [f(0x1e7d30 + 4 * i) for i in range(3)]; fov = f(0x1e7d50)
    st = p._u32(0x422f80) + 0x374
    lim = 2.0 if (p._u32(st & ~3) >> 8 * (st & 3)) & 0xff else 0.8
    h = hide(e, d, fov, lim); n += 1
    if b != prev: print(vs, 'game byte', b, 'eye', [round(x, 2) for x in e], flush=True); prev = b
    if h != bool(b): bad += 1; print(vs, 'MISMATCH rule', h, 'game', b, [round(x, 3) for x in e + d], round(fov, 2), flush=True)
print('frames', n, 'mismatches', bad)
