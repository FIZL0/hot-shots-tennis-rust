# halt the shade build at the first tile's read-back loop and save state 8
import sys, time
sys.path.insert(0, "tools")
from pine import Pine
VSYNC, LOOP = 0x1d5780, 0x33cbfc
p = Pine()
p.load_state(5)
v = p.read32(VSYNC)
while p.read32(VSYNC) - v < 8: time.sleep(0.05)
gm = p.read32(0x422f80)
sh = p.read32(p.read32(gm + 0x84) + 0x138)
orig = p.read32(LOOP)
p.pause(); p.write32(LOOP, 0x1000ffff); p.resume()
p.write32(sh + 0x3c, 1)
time.sleep(3)
pc_wait = p.read32(sh + 0x3c)
print("countdown", pc_wait)
p.pause()
p.save_state(8)
time.sleep(3)
p.write32(LOOP, orig)
p.resume()
print("saved")
