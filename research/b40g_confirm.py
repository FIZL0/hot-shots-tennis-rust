"""B40g: walk the real game to the confirm screen (from save slot 2 via pick.py's path) and screenshot the
Set Handicap / Offbeat Rules sub-screens. Usage (under tools/pcsx2.sh): b40g_confirm.py <out-dir> [extra vpad cmds...]
Saves the confirm screen to scratch slot 9 the first time (pass 'reuse' as the 2nd arg to start from slot 9)."""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

out = sys.argv[1]
p = Pine()
def shot(name):
    subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"], check=True)
if len(sys.argv) > 2 and sys.argv[2] == "reuse":
    p.load_state(9); time.sleep(2.5); cmds = sys.argv[3:]
else:
    p.load_state(2); time.sleep(1.5)
    # P1 Ashley, the three COM picks where their cursors start, Start → confirm screen
    vpad("sleep 2500", "press circle 150", "sleep 1200", *["press circle 150", "sleep 1200"] * 3, "press start 150", "sleep 6000")
    shot("confirm"); p.save_state(9); time.sleep(2)
    cmds = sys.argv[2:]
for k, c in enumerate(cmds):
    if c.startswith("shot:"):
        shot(c[5:])
    elif c.startswith("dump:"):
        a, n = c[5:].split("+")
        print(c, p.read_block(int(a, 16), int(n, 16)).hex(), flush=True)
    else:
        vpad(f"press {c} 150", "sleep 1200")
