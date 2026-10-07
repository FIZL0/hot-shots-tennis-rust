"""P19: screenshots of the original's HUD. Loads a save slot, then sends PCSX2's F8 every <step> vsyncs for <frames>
vsyncs, logging the vsync of each shot (and the match flow state) to <outdir>/shots.txt; PNGs land in PCSX2's snaps.
usage: hud_shots.py <slot> <frames> <step> <outdir>"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, step, out = int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]; os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
log = open(f"{out}/shots.txt", "w")
last, n = p.read32(0x1d5780), 0
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    if n % step == 0:
        f8(); log.write(f"{n} {v}\n"); log.flush()
print("done", n)
