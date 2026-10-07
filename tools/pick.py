#!/usr/bin/env python3
"""Start a doubles match with character <char> (0–13, TParam rows) as P1 and save its first serve to a scratch slot.
Usage (under tools/pcsx2.sh): pick.py <char> [save slot, default 8]  → prints P1's character, hand and model flag.
Loads save slot 2 (doubles character select, cursor on character 0, Ashley), walks the cursor, confirms P1, takes the COM
picks where their cursors start (characters 2, 1, 5), Start, "Start the match!", waits for the match to load (~30 s at 0.5 speed) with P1 serving.
Slot 2's P1 has the character-select "switch hand" toggle on: every pick plays with the hand opposite to TParam's.
Then drive it with tools/vpad.py (serve: circle ×3, the first press after a load is ignored) and look at it with
tools/screenshot.sh. `from pick import pick, vpad` for scripts (research/p7b_record.py)."""
import os, struct, subprocess, sys, time
from pine import Pine

# cursor path from character 0 on the doubles grid (BEG row: 0 1 2 5; INT: 13 | 4 3 / 7 12 | 6; EXP row: 8 9 10 11)
PATH = {0: [], 1: ["right"], 2: ["right"] * 2, 5: ["right"] * 3, 13: ["down"], 4: ["down", "right"],
        3: ["down", "right", "right"], 6: ["down", "right", "right", "right"], 7: ["down", "right", "down"],
        12: ["down", "right", "down", "right"], 8: ["down"] * 2, 9: ["down"] * 2 + ["right"],
        10: ["down"] * 2 + ["right"] * 2, 11: ["down"] * 2 + ["right"] * 3}
T = os.path.dirname(os.path.abspath(__file__))

def vpad(*c):
    # the pad server queues commands and `send` returns at once: wait out the batch so the next step starts after it
    subprocess.run([f"{T}/vpad.py", "send", *c], check=True)
    time.sleep(sum(int(l.split()[-1]) for l in c if l.split()[0] in ("sleep", "press")) / 1000 + 0.1)

def f32(p, a): return struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]

def pick(p, ch, slot=8):
    """Returns (player 0's object address, info dict); exits if the match doesn't start or picked someone else."""
    p.load_state(2)
    time.sleep(1.5)
    vpad("sleep 2500", *[c for d in PATH[ch] for c in (f"press {d} 150", "sleep 700")], "press circle 150",
         "sleep 1200", *["press circle 150", "sleep 1200"] * 3, "press start 150", "sleep 3000")
    t = time.time()
    while True:
        gm = p.read32(0x422f80)
        if not gm and time.time() - t < 20:
            vpad("press circle 150", "sleep 2500")  # "Start the match!" (the confirm screen takes a while to take input)
        pl = gm and p.read32(gm + 0xa8)
        if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
            break
        if time.time() - t > 90: sys.exit("match didn't reach the serve")
        time.sleep(0.1)
    time.sleep(1)
    p.save_state(slot)
    time.sleep(2)
    mdl = p.read32(p.read32(p.read32(pl + 0x54)))
    # sel: P1's selection record (character, costume…, +6 switch hand)
    info = dict(char=p.read32(pl + 0x12bc), hand=f32(p, pl + 0x12b4), flag=p.read8(mdl + 0x135),
                sel=p.read_block(0x2ef7f0, 0x10).hex()[8:], state=p.read8(pl + 0x3fa4))
    if info["char"] != ch: sys.exit(f"picked character {info['char']}, not {ch}: fix PATH[{ch}]")
    return pl, info

if __name__ == "__main__":
    ch, slot = int(sys.argv[1]), int(sys.argv[2]) if len(sys.argv) > 2 else 8
    _, info = pick(Pine(), ch, slot)
    print(info, f"saved to slot {slot}")
