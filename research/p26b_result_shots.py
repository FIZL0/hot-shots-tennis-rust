"""P26b: the original's match-over screens. Loads a save slot, pokes team 0's games to one short of the set
(`games-per-set − 1`), waits for the match to end and logs the result object (0x43b2f0: state +0x217c, page
+0x2364, slide +0x2368/+0x236c, timer +0x237c, blink +0x23d4) every vsync, taking screenshots (F8) at named
moments; presses ✕ once the bar is up to slide to the stats page.
usage: p26b_result_shots.py <slot|-> <outdir> [max_frames]"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
out = sys.argv[2]; os.makedirs(out, exist_ok=True)
want = int(sys.argv[3]) if len(sys.argv) > 3 else 20000
if sys.argv[1] != "-":  # "-": carry on from where the copy is
    p.load_state(int(sys.argv[1])); time.sleep(1)
    g = p.read32(0x423044) - 1
    p.write32(0x42306c, g); p.write32(0x42307c, g)
i32 = lambda a: p.read32(a) - (1 << 32) * (p.read32(a) >> 31)
def shot(name):
    subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"], capture_output=True)
def pad(*cmds):
    subprocess.run(["tools/vpad.py", "send", *cmds])
log = open(f"{out}/log.txt", "w")
last, n, done, prev = p.read32(0x1d5780), 0, set(), None
marks = {(0, 30): "s0_30", (0, 120): "s0_120", (1, 1): "s1_t1", (1, 60): "s1_t60", (1, 125): "s1_t125"}
pressed = 0
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    o = p.read32(0x43b2f0)
    if not o or not (0x100000 <= o < 0x2000000): continue
    st, page, sx, sd, t, bl = p.read8(o + 0x217c), i32(o + 0x2364), i32(o + 0x2368), i32(o + 0x236c), i32(o + 0x237c), i32(o + 0x23d4)
    cur = (st, page, sd)
    if cur != prev:
        log.write(f"{n} obj={o:#x} state={st} page={page} slide={sx} dir={sd} t={t} blink={bl}\n"); log.flush(); prev = cur
        if (st, sd) == (0, 0) and "s0_start" not in done: done.add("s0_start"); shot("s0_start")
    key = (st, t) if st == 1 else (st, n)
    if st == 0:
        k0 = getattr(sys, "_k0", None) or n; sys._k0 = k0; key = (0, n - k0)
    m = marks.get(key)
    if m and m not in done: done.add(m); shot(m); log.write(f"{n} shot {m}\n")
    if st == 1 and t >= 130 and pressed == 0 and page == 0 and sd == 0:
        pressed = 1; pad("press cross 100"); log.write(f"{n} press cross\n")
    if sd and abs(sx) >= 280 and "mid" not in done: done.add("mid"); shot(f"slide_{sx}")
    if st == 1 and page == 1 and sd == 0 and "page1" not in done:
        done.add("page1"); time.sleep(0.5); shot("page1"); time.sleep(0.8); shot("page1b"); break
log.write(f"end {n}\n"); print("done", n, sorted(done))
