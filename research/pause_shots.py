"""P19a: the original's pause menu. Loads slot 3 (P1 human, waiting), dumps RAM before/after Start and takes
screenshots of each menu step. usage: pause_shots.py <out-dir> [vpad steps after Start, e.g. "down" "down"]"""
import sys, time, subprocess
sys.path.insert(0, "tools")
from pine import Pine

out = sys.argv[1]
def vpad(*c):
    subprocess.run(["tools/vpad.py", "send", *c], check=True)
    time.sleep(sum(int(l.split()[-1]) for l in c if l.split()[0] in ("sleep", "press")) / 1000 + 0.1)
def shot(name): subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"])
def dump(slot): p.save_state(slot); time.sleep(1.5)  # RAM: 7z e -so <sstate> eeMemory.bin

p = Pine(); p.load_state(3); time.sleep(1.5)
dump(8)
vpad("press start 100", "sleep 400")
dump(9)
shot("pause_0")
for k, b in enumerate(sys.argv[2:]):
    vpad(f"press {b} 100", "sleep 500")
    shot(f"pause_{k+1}_{b}")
