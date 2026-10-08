"""B23: the original's serve/smash speed readout. Loads a save slot and logs, each vsync the scoreboard's readout
state changes (on +0x534, alpha +0x538, stage +0x548, t +0x544, digits +0x54c.., count +0x558, side +0x55c) with
the shot counter, hitter, hitter's branch and the ball's velocity; F8 at the readout's first and hold frames.
usage: b23_speed_rec.py <slot> <frames> <outdir>"""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
log = open(f"{out}/speed.txt", "w")
sb = p.read32(0x42d6c0)
last, n, prev, shots = p.read32(0x1d5780), 0, None, 0
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    on = p.read8(sb + 0x534)
    st = (on, p.read32(sb + 0x538), p.read8(sb + 0x548), p.read32(sb + 0x544),
          [p.read32(sb + 0x54c + 4 * i) for i in range(3)], p.read32(sb + 0x558), p.read8(sb + 0x55c))
    sh = p.read32(0x423060)
    if st != prev or sh != shots:
        gm = p.read32(0x422f80); ball = p.read32(gm + 0x88); hit = p.read32(0x423058)
        hit = hit - (1 << 32) if hit >= 1 << 31 else hit
        br = p.read8(p.read32(0x423f80) + 0xdd + 8 * hit) if hit >= 0 else -1
        vel = [f(ball + 0x140 + 4 * i) for i in range(3)]
        log.write(f"{n} {v} shots={sh} hitter={hit} branch={br} vel={vel} st={st}\n"); log.flush()
        if on and (prev is None or not prev[0] or (st[2] == 1 and prev[2] == 0)): f8()
    prev, shots = st, sh
print("done", n)
