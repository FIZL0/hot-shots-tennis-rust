#!/usr/bin/env python3
"""B27b: serve_ai.py in lock-step (HST_LOCKSTEP=1, copy at 1x): each frame's pad is set while PCSX2 is paused, then
one FrameAdvance, so host input lag can't shift it. Same sample layout as serve_ai.py (+ the raw pad packet).
Usage: HST_PCSX2=N HST_LOCKSTEP=1 tools/pcsx2.sh serve_ai_step.py <rec.p2m2> <slot> <out.bin> <start> <end>"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../../tools"))
import pine
pine.Pine.require_realtime = lambda self: None
sys.argv, args = sys.argv[:1], sys.argv[1:]
os.environ.pop("HST_LOCKSTEP")
import play_p2m2 as q
os.environ["HST_LOCKSTEP"] = "1"

rec, slot, out, start, end = args[0], int(args[1]), args[2], int(args[3]), int(args[4])
import zipfile
vs = struct.unpack_from("<I", zipfile.ZipFile(rec + "_SaveState.p2s").read("eeMemory.bin"), q.VSYNC)[0]  # the state's own vsync
p, f = q.p, q.frames(rec)
p.load_state(slot); q.tick(q.tick(q.tick(p.read32(q.VSYNC))))
want = {x[17] for x in f[1:] if ~(x[0] | x[1] << 8) >> 9 & 1}
try: q.calibrate(want)
except AssertionError as err:  # flaky low R2 pressures (play_p2m2's known miss): take the nearest calibrated one
    print(err, flush=True)
    for w in want - q.TRIG.keys(): q.TRIG[w] = q.TRIG[min(q.TRIG, key=lambda t: abs(t - w))]
print("calibrated", flush=True)
s = p  # one PINE connection only: switch it to lock-step
s.lockstep = True
import atexit; atexit.register(s.resume)
s.load_state(slot)  # loads running, pauses after
v0 = s.read32(q.VSYNC)
print("loaded, paused at vsync", v0, flush=True)
gm = s.read32(q.GM_PTR)
pls = [s.read32(gm + 0xa8 + 4 * i) for i in range(4)]
ais = [s.read32(pl + 0x80) for pl in pls]
regions = [(0x423048, 0x20)] + [r for a, pl in zip(ais, pls) for r in ((a + 0x50, 0x10), (a + 0x248, 8), (pl + 0x3d70, 0x10),
           (pl + 0x3e50, 8), (pl + 0x3f00, 8), (pl + 0x3f94, 0x18))]
o, v, n = open(out, "wb"), v0, 0
while v < end:
    k = v - vs + 1
    q.put(f[k] if k < len(f) else q.NEUTRAL)
    time.sleep(0.03)  # SDL picks the event up before the step
    w = s.next_frame(v)
    if w != v + 1: print(f"stepped {v} -> {w}", flush=True)
    v = w
    if v >= start:
        o.write(struct.pack("<I", v) + s.read_regions(regions) + q.raw()); n += 1
    if n and n % 600 == 0: print(n, flush=True)
print("done", n)
