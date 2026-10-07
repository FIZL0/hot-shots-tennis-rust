"""P19: the original's "Score to win" line on demand. Loads a slot where P1 (human) serves and waits for input,
pokes the points / games / sets (0x423064.. 2 ints each), lets the serve panel come in and takes screenshots.
usage: gameinfo_poke.py <slot> <out-prefix> <p0,p1> [g0,g1] [s0,s1]"""
import sys, time, subprocess
sys.path.insert(0, "tools")
from pine import Pine

p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(0.5)
for base, arg in zip((0x423064, 0x42306c, 0x423074), sys.argv[3:]):
    for k, v in enumerate(arg.split(",")):
        p.write32(base + 4 * k, int(v))
hud = p.read32(0x42d6c0)
i32 = lambda a: p.read32(a) - (1 << 32) * (p.read32(a) >> 31)
print("points", [i32(0x423064 + 4 * k) for k in range(2)], "games", [i32(0x42306c + 4 * k) for k in range(2)],
      "sets", [i32(0x423074 + 4 * k) for k in range(2)], "server", i32(0x42304c), "court", i32(0x423050),
      "flag400", p.read8(hud + 400), "slots", [i32(0x422fe8 + 4 * k) for k in range(4)], "slide", i32(hud + 0x164), flush=True)
time.sleep(1)
for k in range(3):
    subprocess.run(["tools/screenshot.sh", f"{sys.argv[2]}_{k}.png"])
