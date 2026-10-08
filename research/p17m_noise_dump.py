"""P17m: freeze the costume noise (rate 0), boost its amp ×K, GS-dump a frame (copy N's GSDumpSingleFrame hotkey
bound to F11; SNAPS = its snaps dir); print every deformer's node, phase, prev, freq, amp before and after.
Usage: SNAPS=... tools/pcsx2.sh python3 research/p17m_noise_dump.py [K]"""
import struct, sys, time, os, subprocess
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
K = float(sys.argv[1]) if len(sys.argv) > 1 else 30
p = Pine()
p.load_state(5); time.sleep(3)
ram = b"".join(p.read_block(a, 0x10000) for a in range(0x100000, 0x2000000, 0x10000))
objs = [0x100000 + o for o in range(0, len(ram), 4) if struct.unpack_from("<I", ram, o)[0] == 0x1d0cf8]
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
for o in objs:
    e = p.read32(p.read32(o + 0x14))
    p.write32(e + 0xc, 0)
    p.write32(e + 0x10, struct.unpack("<I", struct.pack("<f", f(e + 0x10) * K))[0])
time.sleep(1)
def state():
    out = []
    for o in objs:
        e = p.read32(p.read32(o + 0x14))
        out.append((p.read16(e + 2), p.read32(o + 0x1c), p.read32(o + 0x34), p.read32(o + 0x30), p.read32(o + 0x40)))
    return out
a = state()
snaps = os.environ["SNAPS"]
before = set(os.listdir(snaps))
pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{os.environ['HST_PCSX2']}.pid")).read().strip()
subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F11", window = "pid:{pid}" }})'], check=True)
for _ in range(60):
    time.sleep(0.5)
    new = set(os.listdir(snaps)) - before
    if new: break
time.sleep(3)
b = state()
print("new", new)
for x, y in zip(a, b):
    print(" ".join(f"{v:x}" for v in x), "|", " ".join(f"{v:x}" for v in y))
