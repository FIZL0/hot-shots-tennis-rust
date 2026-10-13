"""B58: log the original's framed hits (player +0x3f06, effects manager +0xbc) and the pop-up list's entries (as
surprise_rec.py) from a save slot, F8 once while a skull (kind 7 sprite 12) is fully up. Stops `after` frames past the
first skull. usage: b58_skull_rec.py <slot> <frames> <outdir> [after]"""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; after = int(sys.argv[4]) if len(sys.argv) > 4 else 60
os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
OBJ = 0x1529db0  # the pop-up manager in these save states
assert p.read32(OBJ) == 0x1d1cc0, hex(p.read32(OBJ))
log = open(f"{out}/log.txt", "w")
np_ = p.read32(0x422fa4); world = p.read32(0x422f80); fx = p.read32(0x423f80)
pl = [p.read32(world + 0xa8 + 4 * i) for i in range(np_)]
log.write(f"players {np_} {[hex(a) for a in pl]} fx {fx:#x}\n")
last, n, prev, stop, shot = p.read32(0x1d5780), 0, None, None, False
while n < want and (stop is None or n < stop):
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    framed = [p.read_block(a + 0x3f06, 1)[0] for a in pl]
    state = (framed, p.read_block(fx + 0xbc, 1)[0], p.read32(OBJ + 0x84c))
    if state != prev:
        log.write(f"{n} v={v} framed={state[0]} fx.bc={state[1]} skull_who={state[2]:#x}\n"); prev = state
    cnt = min(p.read32(OBJ + 0x6f0), 15)
    blk = p.read_block(OBJ + 0x150, 0x30 * cnt) if cnt else b""
    for i in range(cnt):
        e = blk[0x30 * i:0x30 * i + 0x30]
        timer, = struct.unpack_from("<i", e, 0x10); stage = struct.unpack_from("<b", e, 0x14)[0]
        alpha, = struct.unpack_from("<f", e, 0x18); spr = e[0x1c]; who, off = struct.unpack_from("<ii", e, 0x20)
        kind = e[0x29]
        if kind == 0 and stage != 1 or kind == 7:
            log.write(f"{n} v={v} i={i} kind={kind} spr={spr} who={who} stage={stage} t={timer} a={alpha:.0f} off={off} grade={e[0x28]}\n")
        if kind == 7 and spr == 12:
            stop = stop or n + after
            if not shot and stage == 1 and timer == 20:
                shot = True; f8(); log.write(f"SHOT {n} v={v}\n")
    log.flush()
print("done", n, "skull" if stop else "no skull")
