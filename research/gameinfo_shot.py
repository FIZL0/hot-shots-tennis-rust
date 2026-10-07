"""P19: screenshot the original's "Score to win" line. Loads a slot, waits for a game point with the serve panel in
as soon as it is in (the shot lands ~1 s later), prints the score state and takes a PCSX2 screenshot.
usage: gameinfo_shot.py <slot> <out.png> [skip]   (skip: game points to pass over first)"""
import sys, time, subprocess
sys.path.insert(0, "tools")
from pine import Pine

p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
skip = int(sys.argv[3]) if len(sys.argv) > 3 else 0
i32 = lambda a: p.read32(a) - (1 << 32) * (p.read32(a) >> 31)
t0, armed = time.time(), True
while time.time() - t0 < 900:
    hud = p.read32(0x42d6c0)
    pts = [i32(0x423064), i32(0x423068)]
    gp = max(pts) > 2 and p.read8(0x316620) == 0
    if not gp:
        armed = True
        continue
    if armed and i32(hud + 0x164) == 5 and i32(hud + 0x1a8) <= 2:
        armed = False
        if skip:
            skip -= 1
            continue
        gm = p.read32(0x422f80)
        print("points", pts, "games", [i32(0x42306c), i32(0x423070)], "sets", [i32(0x423074), i32(0x423078)],
              "server", i32(0x42304c), "court", i32(0x423050), "flag400", p.read8(hud + 400), "adv", p.read8(0x316628),
              "tb", p.read8(0x31661a), "set games", i32(gm + 0x54), "sets", i32(gm + 0x50), flush=True)
        subprocess.run(["tools/screenshot.sh", sys.argv[2]])
        subprocess.run(["tools/screenshot.sh", sys.argv[2].replace(".png", "_b.png")])
        break
