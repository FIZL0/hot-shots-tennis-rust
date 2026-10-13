#!/usr/bin/env python3
"""B77: what pausing does to sound. Loads a slot, lets it run, presses Start, records every sound-ring command
(cmd, voice, word8, wordC) with wall-clock time and a marker for each Start, then Start again.
usage: pause_sound.py <slot> <out.txt> [run_s=4] [paused_s=4] [after_s=3]"""
import struct, sys, time, subprocess
sys.path.insert(0, "tools")
from pine import Pine

RING, HEAD, VSYNC = 0x305000, 0x304fc0, 0x1d5780
slot, out = int(sys.argv[1]), open(sys.argv[2], "w")
run_s, paused_s, after_s = (float(x) for x in (sys.argv[3:6] + ["4", "4", "3"][len(sys.argv[3:6]):]))
p = Pine()
p.load_state(slot)
time.sleep(0.5)
head, t0 = p.read32(HEAD) % 1024, time.time()

def poll(secs):
    global head
    end = time.time() + secs
    while time.time() < end:
        h = p.read32(HEAD) % 1024
        k = (h - head) % 1024
        if k:
            b = p.read_regions([(RING + 16 * head, 16 * min(k, 1024 - head)), (RING, 16 * max(0, head + k - 1024))])
            v = p.read32(VSYNC)
            for i in range(k):
                c = struct.unpack_from("<4I", b, 16 * i)
                out.write(f"{time.time() - t0:7.3f} v{v} cmd {c[0]:2} voice {c[1]:#x} {c[2]:#010x} {c[3]:#010x}\n")
        head = h
        time.sleep(0.005)

def start(tag):
    out.write(f"{time.time() - t0:7.3f} --- START {tag}\n")
    subprocess.Popen(["tools/vpad.py", "send", "press start 100"])

poll(run_s); start("open"); poll(paused_s); start("close"); poll(after_s)
print("done")
