"""P12b2: screenshots of the original's game / set / tiebreak-point shows. Loads a save slot, pokes the score to
games `g0`-`g1` with two sets to win (so a set win is not the match), polls the scoreboard every vsync and sends
PCSX2's F8 at chosen moments of kinds 3 (game), 4 (set) and 6 (tiebreak point), logging the state to <outdir>/shots.txt.
usage: popup_shots2.py <slot> <max_frames> <outdir> [g0 g1]"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
if len(sys.argv) > 5:
    g = int(sys.argv[4]), int(sys.argv[5])
    p.write32(0x423048, 2)
    for t in range(2):
        p.write32(0x42306c + 4 * t, g[t]); p.write32(0x42307c + 0x14 * t, g[t])
    if g == (6, 6):  # tiebreak flag, as the scoring routine sets it at 6-6 (a byte: keep its neighbours)
        w = p.read32(0x316618); p.write32(0x316618, w & ~0xff0000 | 0x10000)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
log = open(f"{out}/shots.txt", "w")
sb = p.read32(0x42d6c0)
last, n, taken = p.read32(0x1d5780), 0, set()
def moment(kind, stage, step, t, fade):
    """A name for the frames worth a shot, else None."""
    if fade == 2: return "out" if t == 2 else None
    if kind in (3, 4):
        if fade == 0: return "in" if t == 7 else None
        if stage == 0 and step in (3, 6): return f"rise{step}"
        if stage == 1 and step in (8, 3): return f"drop{step}"
        return "settled" if stage == 2 else None
    if fade == 0: return None
    if stage == 1 and step == 4: return "slide4"
    if stage == 2: return f"swap{t}"
    if stage == 3 and t == 7: return "flash7"
    return "settled" if stage == 4 else None
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    kind = p.read8(sb + 0x148); stage = p.read32(sb + 0x168); step = p.read32(sb + 0x164); t = p.read32(sb + 0x154)
    fade = p.read8(sb + 0x150); done = p.read8(sb + 0x151)
    if p.read8(0x4230be): break  # match over: later frames are the post-match screens
    if kind not in (3, 4, 6) or done: continue
    hit = moment(kind, stage, step, t, fade)
    if hit is None or (kind, hit) in taken: continue
    taken.add((kind, hit)); f8(); print(hit, kind, flush=True)
    g = [p.read32(0x42306c + 4 * i) for i in range(2)]; hist = [p.read32(0x42307c + 4 * i) for i in range(10)]
    log.write(f"{n} {v} {hit} kind={kind} stage={stage} step={step} t={t} fade={fade} alpha={p.read32(sb+0x18c)} pts={(p.read32(0x423064), p.read32(0x423068))} "
              f"games={g} sets={(p.read32(0x423074), p.read32(0x423078))} hist={hist} set={p.read32(0x4230a4)} scorer={p.read32(0x4230a8)} "
              f"swap={p.read8(sb+400)} deuce={(p.read8(0x316620), p.read8(0x316628))} tb={p.read8(0x31661a)} over={p.read8(0x4230be)}\n"); log.flush()
    if {k for k, _ in taken} >= {3, 4, 6}: break
print("done", n, len(taken))
