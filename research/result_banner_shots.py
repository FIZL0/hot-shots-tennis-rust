"""P12b5: screenshots of the original's "Game/Set" + "Server/Receiver" banner over the result board. Loads a save
slot, pokes games `g0`-`g1` (two sets to win), polls the scoreboard's banner (flag +0x571, timer +0x574, stage
+0x578) every vsync, sends F8 at named moments and logs the state to <outdir>/shots.txt.
usage: result_banner_shots.py <slot> <max_frames> <outdir> [g0 g1]"""
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
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
log = open(f"{out}/shots.txt", "w")
sb = p.read32(0x42d6c0)
last, n, taken, banners, prev = p.read32(0x1d5780), 0, set(), 0, 0
while n < want and banners < 2:  # one banner (held ends it)
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    on = p.read8(sb + 0x571); t = p.read32(sb + 0x574); stage = p.read8(sb + 0x578)
    if p.read8(0x4230be): break
    if not on:
        prev = 0; continue
    if not prev:
        banners += 1; taken = set()
        log.write(f"{n} start wait={p.read32(sb+0x184)} kind={p.read8(sb+0x148)} t={t}\n")
    prev = 1
    hit = {(0, 13): "s0t13", (0, 7): "s0t7", (0, 0): "s0t0", (1, 16): "s1t16", (1, 8): "s1t8", (2, 0): "held"}.get((stage, t))
    if hit is None or hit in taken: continue
    taken.add(hit); f8(); print(banners, hit, flush=True)
    if hit == "held": banners = 2
    log.write(f"{n} {banners} {hit} stage={stage} t={t} server={p.read32(0x42304c)} winner={p.read32(0x4230a8)} result={p.read32(0x4230b8)} "
              f"wait={p.read32(sb+0x184)} kind={p.read8(sb+0x148)} games={[p.read32(0x42306c + 4 * i) for i in range(2)]}\n"); log.flush()
print("done", n, banners)
