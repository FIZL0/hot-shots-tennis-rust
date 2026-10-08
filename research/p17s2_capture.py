"""P17s2: GS-dump a frame of a save slot (default 4: court 4, whose court/model/player third lights are non-zero) and
save EE RAM right after it, so the dump's court and character colours can be checked against the light blocks that
drew them (research/p17s2_check.py).  SNAPS=<pcsx2 snaps dir> p17s2_capture.py [slot] [outdir]"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
slot = int(sys.argv[1]) if len(sys.argv) > 1 else 4
out = sys.argv[2] if len(sys.argv) > 2 else "context/p17s2"
p = Pine()
p.load_state(slot); time.sleep(4)
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
p.pause()
ram = b"".join(p.read_block(a, 0x10000) for a in range(0, 0x2000000, 0x10000))
p.resume()
open(f"{out}/s{slot}.ram", "wb").write(ram)
print("new", new)
