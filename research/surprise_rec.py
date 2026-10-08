"""P24: log the original's head pop-ups. Loads a save slot and every vsync reads the pop-up list (count +0x6f0,
0x30-byte entries from +0x150: kind +0x29, sprite +0x1c, stage +0x14, timer +0x10, alpha +0x18, player +0x20) and
each player's surprise timer (+0x3a90) and controller id (+0x13f0, 0x21 = CPU). Logs each new entry, and every frame
of the surprise pop-ups (kind 7 "!", kind 1 sweat / "...", kind 6 swirl), to <outdir>/log.txt; sends PCSX2's F8 at
chosen moments of the first `shots` of each sprite (logged as SHOT lines; the PNGs land in PCSX2's snaps folder).
usage: surprise_rec.py <slot> <frames> <outdir> [shots]"""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; shots = int(sys.argv[4]) if len(sys.argv) > 4 else 0
os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
OBJ = 0x1529db0  # the pop-up manager in these save states
assert p.read32(OBJ) == 0x1d1cc0, hex(p.read32(OBJ))
log = open(f"{out}/log.txt", "w")
np_ = p.read32(0x422fa4); world = p.read32(0x422f80)
pl = [p.read32(world + 0xa8 + 4 * i) for i in range(np_)]
log.write(f"players {np_} {[hex(a) for a in pl]}\n")
MOMENTS = {(0, 1), (1, 20), (1, 0), (2, 1), (1, 60)}  # (stage, timer) worth a shot (kind 1 holds at t=60)
last, n, seen, taken, done = p.read32(0x1d5780), 0, set(), set(), {}
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    cnt = min(p.read32(OBJ + 0x6f0), 15)
    blk = p.read_block(OBJ + 0x150, 0x30 * cnt) if cnt else b""
    sur = [struct.unpack_from("<ii", p.read_block(a + 0x3a90, 8))[0] for a in pl]
    for i in range(cnt):
        e = blk[0x30 * i:0x30 * i + 0x30]
        timer, = struct.unpack_from("<i", e, 0x10); stage = struct.unpack_from("<b", e, 0x14)[0]
        alpha, = struct.unpack_from("<f", e, 0x18); spr = e[0x1c]; who, off = struct.unpack_from("<ii", e, 0x20)
        kind = e[0x29]
        key = (kind, spr, who)
        if kind in (1, 6, 7) or key not in seen:
            log.write(f"{n} v={v} i={i} kind={kind} spr={spr} who={who} stage={stage} t={timer} a={alpha:.0f} off={off} grade={e[0x28]} sur={sur}\n")
        seen.add(key)
        if kind in (1, 7) and done.get(spr, 0) < shots and (stage, timer) in MOMENTS:
            tag = (spr, done.get(spr, 0), stage, timer)
            if tag not in taken:
                taken.add(tag); f8(); log.write(f"SHOT {n} v={v} spr={spr} who={who} stage={stage} t={timer}\n")
                if (stage, timer) == (2, 1) or (kind == 1 and stage == 1): done[spr] = done.get(spr, 0) + 1
    log.flush()
print("done", n)
