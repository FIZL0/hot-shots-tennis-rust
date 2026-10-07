"""P12b: screenshots of the original's point pop-up. Loads a save slot, polls the scoreboard (kind, stage, roll step)
every vsync and sends PCSX2's F8 at chosen moments of the first point show, logging the state to <outdir>/shots.txt.
usage: popup_shots.py <slot> <max_frames> <outdir>"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
log = open(f"{out}/shots.txt", "w")
sb = p.read32(0x42d6c0)
last, n, taken = p.read32(0x1d5780), 0, set()
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    kind = p.read8(sb + 0x148); stage = p.read32(sb + 0x168); step = p.read32(sb + 0x164); t = p.read32(sb + 0x154)
    fade = p.read8(sb + 0x150); done = p.read8(sb + 0x151)
    pts = (p.read32(0x423064), p.read32(0x423068)); deuce = (p.read8(0x316620), p.read8(0x316628), p.read32(0x316624))
    key = (kind, stage, step if stage == 1 else 0, fade)
    if kind == 1 and not done and key not in taken and (stage in (1, 4) and fade == 1):
        taken.add(key); f8()
        log.write(f"{n} {v} kind={kind} stage={stage} step={step} t={t} fade={fade} pts={pts} deuce={deuce} swap={p.read8(sb+400)} scorer={p.read32(sb+0x15c)}\n"); log.flush()
    if len(taken) >= 7: break
print("done", n, len(taken))
