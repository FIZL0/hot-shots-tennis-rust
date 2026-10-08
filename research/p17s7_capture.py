"""P17s7: GS-dump the moving clouds. No save state is a singles match, so in slot 5 (court 10 doubles) this sets the
court manager's singles flag (clouds tick and draw), and holds the clouds (camera-relative) into the
match camera's view (rewritten until the dump lands) at radii giving fades from 1 down to ~0.1, then GS-dumps (F11) and saves EE RAM.
  SNAPS=<pcsx2 snaps dir> p17s7_capture.py [outdir]"""
import sys, time, os, struct, subprocess, math
sys.path.insert(0, "tools")
from pine import Pine
out = sys.argv[1] if len(sys.argv) > 1 else "context/p17s7"
p = Pine()
f2u = lambda x: struct.unpack("<I", struct.pack("<f", x))[0]
u2f = lambda v: struct.unpack("<f", struct.pack("<I", v))[0]
p.load_state(5)
time.sleep(2)  # the load lands late (it can roll the vsync counter back)
p.pause()
env = p.read32(p.read32(0x422f80) + 0x84)
p.write32(env + 0x1a14, f2u(0.0))           # wind speed 0: the clouds hold still
b = p.read32(env + 0x134) & ~0xff0000 | 0x10000  # +0x136 = 1: singles
p.write32(env + 0x134, b)
fw = [u2f(p.read32(0x1e7d30 + 4 * i)) for i in range(3)]  # main view forward (game space, y down)
c, k, info = p.read32(0x423208), 0, []
radii = [1500, 1780, 1820, 1860, 1890, 1920, 1950, 1970, 1990]
while c and k < len(radii):
    r, a = radii[k], math.radians(-12 + 3 * k)
    x, z = r * math.sin(a), r * math.cos(a)
    y = r * fw[1] / math.hypot(fw[0], fw[2])
    info.append((c, r, x, y, z))
    c, k = p.read32(c + 0x144), k + 1
def place():  # the wind speed is rewritten every frame, so the clouds are put back until the dump lands
    p.write32(env + 0x1a14, f2u(0.0))
    for c, r, x, y, z in info:
        for o, val in ((0x150, x), (0x154, y), (0x158, z)): p.write32(c + o, f2u(val))
place()
p.resume()
print("moved", [(hex(c), r) for c, r, *_ in info], flush=True)
for _ in range(30): place(); time.sleep(0.05)
snaps = os.environ["SNAPS"]
before = set(os.listdir(snaps))
pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{os.environ['HST_PCSX2']}.pid")).read().strip()
subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F11", window = "pid:{pid}" }})'], check=True)
new = set()
for _ in range(600):
    place(); time.sleep(0.05)
    new = set(os.listdir(snaps)) - before
    if new: break
for _ in range(60): place(); time.sleep(0.05)
p.pause()
ram = b"".join(p.read_block(a, 0x10000) for a in range(0, 0x2000000, 0x10000))
p.resume()
open(f"{out}/s5.ram", "wb").write(ram)
print("new", new)
