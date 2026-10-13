#!/usr/bin/env python3
"""B73: the head markers' show flag (balloon manager +0x725) across a change of ends.
Loads <slot>, waits for the change-ends phase (gm+0x55 == 1), then prints every change of (phase.sub, HUD mode,
panel slide, marker flag) from 40 vsyncs before it until <after> vsyncs after it ends, with the phase tick (+0x58).
A screenshot at tick 20 of phase <shotph> (1 change ends, 2 the serve after it) into <outdir> (blocking: rows may be missed after it).
usage: b73_marker_ends.py <slot> [after] [cap] [outdir]"""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine


def find(p):
    """The balloon manager: the one object whose vtable word is 0x1d1cc0 (as marker_rec.py)."""
    for base in range(0x400000, 0x2000000, 0x10000):
        b = p.read_block(base, 0x10000)
        i = b.find(struct.pack("<I", 0x1d1cc0))
        while i >= 0:
            if i % 4 == 0:
                return base + i
            i = b.find(struct.pack("<I", 0x1d1cc0), i + 1)


if __name__ == "__main__":
    p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
    after_max = int(sys.argv[2]) if len(sys.argv) > 2 else 200
    cap = int(sys.argv[3]) if len(sys.argv) > 3 else 40000
    out = sys.argv[4] if len(sys.argv) > 4 else '.'
    SHOT_PH = int(sys.argv[5]) if len(sys.argv) > 5 else 1  # 1: change-ends tick 20, 2: the next serve's tick 20
    gm = p.read32(0x422f80); bm = find(p); hud = p.read32(0x42d6c0)
    last, n, prev, hist, seen, after = p.read32(0x1d5780), 0, None, [], False, -1
    while n < cap:
        v = p.next_frame(last); last = v; n += 1
        g = p.read_block(gm + 0x54, 8); h = p.read_block(hud + 0x148, 0x20)
        row = (f"{g[1]}.{g[2]}", h[0], struct.unpack("<i", h[0x1c:0x20])[0], p.read_block(bm + 0x724, 4)[1])
        line = f"{v} t{struct.unpack('<I', g[4:8])[0]} ph {row[0]} hud {row[1]} slide {row[2]} marker {row[3]}"
        if g[1] == 1 and not seen:
            seen = True
            print("\n".join(hist), flush=True)
        if not seen:
            if row != prev: hist = (hist + [line])[-40:]
        else:
            if row != prev: print(line, flush=True)
            t = struct.unpack('<I', g[4:8])[0]
            if g[1] == SHOT_PH and t == 20 and (SHOT_PH == 1 or after < 40):
                shot = os.path.join(out, f"ph{g[1]}_t{t:02d}.png")
                if not os.path.exists(shot):  # one at a time: the copy's snaps folder holds only the newest
                    print(v, "shot", shot, flush=True); subprocess.run(["tools/screenshot.sh", shot])
            if g[1] != 1:
                after += 1
                if after >= after_max and g[1] != 2: break
        prev = row
    print("done", n, "vsyncs, seen", seen)
