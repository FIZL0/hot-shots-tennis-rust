#!/usr/bin/env python3
"""P5: record P1's (human) presses from save slot 4 with the virtual pad in the `frames_live` sample format
(hst_sim::replay), for the shot-press decode test: ✕ held for seconds, ✕+○ / ○+△ / all three on one frame,
double taps 6 and 20 frames apart, single taps of each, d-pad diagonals held. The schedule repeats every CYCLE
frames, counted in recorded frames so it lands alike at any speed.
Usage: tools/pcsx2.sh python3 research/p5_input_rec.py <slot> <out.bin> <frames>"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
CYCLE = 600
# (frame in cycle, command)
SCHEDULE = [
    (10, "down cross"), (190, "up cross"),                                          # held ✕
    (230, "down cross"), (230, "down circle"), (233, "up cross"), (233, "up circle"),  # ✕+○ one frame
    (280, "down up"), (280, "down right"), (340, "up up"), (340, "up right"),        # d-pad diagonal
    (360, "down circle"), (363, "up circle"), (366, "down circle"), (369, "up circle"),  # ○ double tap, 6 apart
    (420, "down circle"), (420, "down triangle"), (423, "up circle"), (423, "up triangle"),  # ○+△
    (470, "down down"), (470, "down left"), (520, "up down"), (520, "up left"),      # d-pad diagonal
    (530, "down triangle"), (533, "up triangle"), (550, "down triangle"), (553, "up triangle"),  # △ twice, 20 apart
    (580, "down cross"), (580, "down circle"), (580, "down triangle"),
    (583, "up cross"), (583, "up circle"), (583, "up triangle"),                     # all three
]

pad = os.open(FIFO, os.O_RDWR)
def send(*cmds): os.write(pad, ("\n".join(cmds) + "\n").encode())

p, out, want = Pine(step=True), open(sys.argv[2], "wb"), int(sys.argv[3])
send("release")
p.load_state(int(sys.argv[1]))
time.sleep(2)
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
r = [PAD, GLOBALS, (gm, 0x100), (p.read32(gm + 0x98), 0x290)]
r += [(pl + o, n) for pl in players for o, n in PLAYER] + [(p.read32(gm + 0x88), 0x290), RALLY]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    cmds = [c for f, c in SCHEDULE if f == n % CYCLE]
    if cmds: send(*cmds)
send("release")
out.close()
print("done", n)
