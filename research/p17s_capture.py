"""P17s: GS-dump a frame of slot 5 and read the scene light block (0x1e7d70..0x1e8170) before and after it, so the
dump's character/ball colours can be checked against the light that drew them (research/p17s_light_gs.py)."""
import struct, sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
p.load_state(5); time.sleep(4)
def block():
    b = p.read_block(0x1e7d70, 0x400)
    f = lambda a: struct.unpack_from("<4f", b, a - 0x1e7d70)
    return {hex(a): tuple(round(x, 5) for x in f(a)) for a in (0x1e7d70, 0x1e7d80, 0x1e7d90, 0x1e8130, 0x1e8140, 0x1e8150, 0x1e8160)}
a = block()
snaps = os.environ["SNAPS"]
before = set(os.listdir(snaps))
pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{os.environ['HST_PCSX2']}.pid")).read().strip()
subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F11", window = "pid:{pid}" }})'], check=True)
new = set()
for _ in range(60):
    time.sleep(0.5)
    new = set(os.listdir(snaps)) - before
    if new: break
time.sleep(3)
b = block()
print("new", new)
for k in a:
    print(k, a[k], "|", b[k])
