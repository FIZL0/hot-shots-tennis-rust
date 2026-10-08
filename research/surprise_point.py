"""P24: when the sweat drop comes and goes. Loads <slot> (or, with skip 0, a state saved by an earlier run), runs
<skip> frames and saves state <save>, then logs <frames> frames of the judge's call (scoreboard +0x426), the pop-up
list's kinds/sprites/players and the vsync, and sends F8 the frame the sweat drop appears and the frames around the
one it goes. usage: surprise_point.py <slot> <skip> <save> <frames> <outdir>"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
slot, skip, save, want, out = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
p = Pine(); p.load_state(slot); time.sleep(1)
os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
OBJ, SB = 0x1529db0, p.read32(0x42d6c0)
last, n = p.read32(0x1d5780), 0
def frame():
    global last
    while (v := p.read32(0x1d5780)) == last: pass
    last = v
    return v
for _ in range(skip): frame()
if skip: p.save_state(save); time.sleep(1); last = p.read32(0x1d5780)
log, had = open(f"{out}/point.txt", "w"), False
for n in range(want):
    v = frame()
    cnt = min(p.read32(OBJ + 0x6f0), 15)
    blk = p.read_block(OBJ + 0x150, 0x30 * cnt) if cnt else b""
    pops = [(blk[0x30 * i + 0x29], blk[0x30 * i + 0x1c], blk[0x30 * i + 0x20], blk[0x30 * i + 0x14]) for i in range(cnt)]
    sweat = any(k == 1 for k, *_ in pops)
    if sweat != had: f8(); log.write(f"SHOT {'on' if sweat else 'off'}\n")
    had = sweat
    log.write(f"{n} v={v} call={p.read8(SB + 0x426)} pops={pops}\n")
    log.flush()
