#!/usr/bin/env python3
"""B479: can a doubles player still take a ball its partner is going for? Loads slot <slot> (P1 human beside a
CPU partner), drives P1 with tools/vpad.py and logs every frame: match phase gm+0x55, rally shot count, last hitter,
ball pos, and per player its branch +0x3ec1, countdown +0x3ec4, cancel flag +0x3ec8, motion id (+0x54)->+0x20 and
position. Modes:
  lock  once P1's partner (player 2) locks onto an opponent's ball (countdown > 0), P1 is put at the partner's z,
        mirrored across the partner's contact x, and presses ○ (does P1 lock too, who strikes, is the other cancelled)
  dive  on each opponent hit coming to P1's end, P1 is put <gap> m beside the CPU partner (player 2) on its z,
        runs toward it (across the ball's path) and presses ○ when the ball is <lead> frames from the partner's z,
        so it dives (branch 3; countdown -3 when it falls short): does the partner still lock and strike?
Usage: b479_partner_ball.py <slot> lock|dive <frames> [gap m] [lead frames] > log. Needs tools/vpad.py serve."""
import os, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR, SHOTS, HITTER = 0x1d5780, 0x422f80, 0x423060, 0x423058
slot, mode, want = int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
gap = float(sys.argv[4]) if len(sys.argv) > 4 else 3.5
lead = int(sys.argv[5]) if len(sys.argv) > 5 else 12
f2u = lambda f: struct.unpack("<I", struct.pack("<f", f))[0]
s32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
F = lambda b, o, n=3: struct.unpack_from(f"<{n}f", b, o)
pad = os.open(FIFO, os.O_RDWR)
send = lambda c: os.write(pad, (c + "\n").encode())
p = Pine()
p.load_state(slot)
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
teams = [p.read32(a + 0x12b8) & 1 for a in players]
print("teams", teams, flush=True)
last, n, prev_hitter, fired, stage = p.read32(VSYNC), 0, None, None, None
while n < want:
    v = p.next_frame(last)
    last = v
    b = p.read_block(ball, 0x140)
    hitter = s32(p.read32(HITTER))
    pl = []
    for a in players:
        blk = p.read_block(a + 0x3ec0, 0x10)
        pl.append((blk[1], struct.unpack_from("<i", blk, 4)[0], blk[8], p.read32(p.read32(a + 0x54) + 0x20),
                   F(p.read_block(a + 0x3d70, 0x10), 0), F(p.read_block(a + 0x3fb0, 0x10), 0)))
    ph = p.read8(gm + 0x55)
    line = (f"{v} ph {ph} shots {s32(p.read32(SHOTS))} hitter {hitter} ball {['%.2f' % c for c in F(b, 0xe0)]} "
            + " ".join(f"P{i}[{br} {cd} {cx} m{m:#x} {pos[0]:.2f},{pos[2]:.2f}]" for i, (br, cd, cx, m, pos, _) in enumerate(pl)))
    opp = hitter >= 0 and (hitter & 1) != teams[0]
    if mode == "lock" and fired is None and ph == 3 and opp:
        br, cd, _, _, pos, con = pl[2]
        if br in (1, 2, 4) and cd > 2:
            x = con[0] + (con[0] - pos[0])
            for k, c in ((0, x), (2, pos[2])):
                p.write32(players[0] + 0x3d70 + 4 * k, f2u(c))
            send("press circle 60")
            fired = v
            line += f"  <- P2 locked {cd} (branch {br}) contact x {con[0]:.2f}: P1 put at x {x:.2f}, z {pos[2]:.2f}, presses"
    if mode == "dive" and ph == 3:
        bx, by, bz = F(b, 0xe0)
        vx, _, vz = F(b, 0x130)
        _, _, _, _, mate, _ = pl[2]
        if opp and hitter != prev_hitter and vz * mate[2] > 0:
            # level with where the ball crosses the partner's z, on the side away from the partner, far enough
            # that after running ~0.11 m a frame it is <gap> m short when it presses <lead> frames out
            t = (mate[2] - bz) / vz
            cx = bx + vx * t
            s_ = 1.0 if cx >= mate[0] else -1.0
            x = cx + s_ * (gap + 0.11 * max(t - lead - 3, 0))
            for k, c in ((0, x), (2, mate[2])):
                p.write32(players[0] + 0x3d70 + 4 * k, f2u(c))
            sx = -s_ * (1 if mate[2] < 0 else -1)  # stick x is the court's x on the near (−z) end
            send(f"stick l {sx} 0")
            stage = ("wait", mate[2])
            line += f"  <- opponent hit: crosses x {cx:.2f} in {t:.1f}; P1 put at x {x:.2f}, z {mate[2]:.2f}, runs (stick {sx})"
        elif stage and stage[0] == "wait" and vz != 0 and (stage[1] - bz) / vz <= lead:
            send("press circle 60")
            line += f"  <- P1 presses ({(stage[1] - bz) / vz:.1f} frames out) {abs(bx - pl[0][4][0]):.2f} m off the ball's x"
            stage = ("pressed", v)
            fired = fired or v
        elif stage and stage[0] == "pressed" and v - stage[1] > 30:
            send("stick l 0 0")
            stage = None
    prev_hitter = hitter
    if fired is not None:
        n += 1
    print(line, flush=True)
send("release")
print("done", n)
