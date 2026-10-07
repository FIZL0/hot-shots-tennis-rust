#!/usr/bin/env python3
"""Play a PCSX2 input recording (.p2m2) into PCSX2 copy N through its virtual pad and capture it, unattended — for
when nobody can click Tools → Input Recording → Play (record_p2m2.py waits for that).
Usage: HST_PCSX2=N play_p2m2.py <rec.p2m2> <slot> <out.bin> [frames]
  slot: the recording's save state copied into copy N's sstates as that slot.
Copy N must run slowed (NominalScalar = 0.25) with tools/vpad.py serve up (tools/pcsx2-hst.sh does that).
PCSX2's own playback, at each vsync: poll host input, frame counter += 1, override port 0 with frame[counter]. Here:
on seeing the game's vsync counter at v (state loaded at v0), write frame[v - v0 + 1] to the pad's event node, so the
next vsync's poll picks it up — the same frame. Sticks and R2 pressure are written as raw axis values calibrated
first, so PCSX2 reports the recorded bytes. Port 1 must be idle (1-human recordings).
Writes out.bin in record_p2m2.py's layout and out.bin.pad: per sample, the raw pad packet the game received
u32 vsync + (buttons, rx, ry, lx, ly, 12 pressures); the run fails unless that equals the recording on every frame.
Also out.bin.mark: per sample u32 vsync + the smash (yellow) marker object (match manager +0xa4) bytes +0x50..+0x1b0,
and out.bin.path: each frame the marker's count +0x144 turns non-zero, u32 vsync, u32 len + its predicted path
(len × 0x30 bytes: pos, vel, …, bounces at +0x20); and out.bin.setup.json: the menu's player slots (0x2ef7f4, 12 bytes each: character, outfit, …,
setup word at +8), the marker's per-slot character table (+400) each player's AI object (player +0x80, 24 bytes)
and hand (player +0x12b4, f32: +1 right, −1 left)."""
import json, os, struct, sys, time
from evdev import InputDevice, list_devices, ecodes as e
from pine import Pine

# record_p2m2.py's sample layout (that file only runs as a script)
VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, GM, BALL = (0x2efb00, 0x90), (0x422f80, 0x180), 0x100, 0x290
PLAYER, RALLY = ((0x1380, 0x200), (0x3c00, 0x400)), (0x3165f0, 0x50)
def regions(p):
    gm = p.read32(GM_PTR)
    r = [PAD, GLOBALS, (gm, GM), (p.read32(gm + 0x98), BALL)]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        r += [(pl + o, n) for o, n in PLAYER]
    return gm, r + [(p.read32(gm + 0x88), BALL), RALLY]

RAW = 0x2efc10  # pad manager's raw packet buffer, port 0 at +6 (the bytes PCSX2's pad sent)
INST = os.environ["HST_PCSX2"]

def frames(path):
    d = open(path, "rb").read()
    n = struct.unpack_from("<i", d, 561)[0] + 1
    f = [d[570 + 36 * i:570 + 36 * i + 36] for i in range(n)]
    assert all(x[18:] == bytes(18) for x in f), "port 1 not idle"
    return [x[:18] for x in f]

dev = next(d for d in map(InputDevice, list_devices()) if d.name == f"HST vpad {INST}")
BTN = {14: e.BTN_A, 13: e.BTN_B, 12: e.BTN_Y, 15: e.BTN_X, 3: e.BTN_START, 1: e.BTN_THUMBL,
       10: e.BTN_TL, 11: e.BTN_TR}  # PS2 button bit (active low) → pad button; R2 is the trigger
def stick(b):
    """axis value PCSX2 (AxisScale 1.33, no deadzone) turns into byte b: 127 + (r+1)/2 or 127 - r/2, r = ⌊v·1.33·255⌋"""
    if b == 127: return 0
    r = 2 * (b - 127) - 1 if b > 127 else 2 * (127 - b)
    v = (r + 0.5) / (1.33 * 255)
    return min(32767, round(v * 32767)) if b > 127 else -min(32768, round(v * 32768))
NEUTRAL = bytes([0xff, 0xff, 127, 127, 127, 127]) + bytes(12)
TRIG = {}  # R2 pressure → trigger value, calibrated

def put(f):
    w = ~(f[0] | f[1] << 8) & 0xffff
    assert not w & ~(sum(1 << b for b in BTN) | 1 << 9), f"unsupported buttons {w:#x}"
    for b, k in BTN.items(): dev.write(e.EV_KEY, k, w >> b & 1)
    dev.write(e.EV_ABS, e.ABS_RZ, TRIG[f[17]] if w >> 9 & 1 else 0)
    for a, x in zip((e.ABS_RX, e.ABS_RY, e.ABS_X, e.ABS_Y), f[2:6]): dev.write(e.EV_ABS, a, stick(x))
    dev.syn()

p = Pine()
def tick(v):
    while (w := p.read32(VSYNC)) == v: pass
    return w
def raw(): return p.read_block(RAW, 24)[6:24]

def calibrate(want):
    """stick bytes on both axes and the R2 pressures in `want`; each stick byte must show up within six frames (host
    input lag jitters), each pressure is read two frames after it is set"""
    for b in range(256):
        put(NEUTRAL[:4] + bytes([b, 255 - b]) + bytes(12)); v = p.read32(VSYNC)
        for _ in range(6):
            v = tick(v)
            if raw()[4:6] == bytes([b, 255 - b]): break
        else: sys.exit(f"stick byte {b}: PCSX2 reports {raw()[4:6].hex()}")
    for t in range(1, 256):
        dev.write(e.EV_ABS, e.ABS_RZ, t); dev.syn(); tick(tick(p.read32(VSYNC)))
        TRIG.setdefault(raw()[17], t)
    dev.write(e.EV_ABS, e.ABS_RZ, 0); dev.syn()
    assert want <= TRIG.keys(), f"no trigger value gives R2 pressure {sorted(want - TRIG.keys())}"

if __name__ == "__main__":
    rec, slot, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    f = frames(rec)
    want = len(f) if len(sys.argv) < 5 else int(sys.argv[4])
    if p.status() != "running": sys.exit("PCSX2 copy is paused")
    p.load_state(slot); tick(tick(tick(p.read32(VSYNC))))  # calibrate in the match: other screens skip pad polls
    calibrate({x[17] for x in f[1:] if ~(x[0] | x[1] << 8) >> 9 & 1})
    print("calibrated", flush=True)
    put(f[1])
    last = p.read32(VSYNC)
    p.load_state(slot)
    while abs((v0 := p.read32(VSYNC)) - last) < 3: last = v0
    print("loaded at vsync", v0, flush=True)
    gm = p.read32(GM_PTR); m = p.read32(gm + 0xa4)
    json.dump({"slots": p.read_block(0x2ef7f0, 0x38)[4:52].hex(), "marker_chars": p.read_block(m + 400, 16).hex(),
               "ai": [p.read_block(p.read32(p.read32(gm + 0xa8 + 4 * i) + 0x80), 24).hex() for i in range(4)],
               "hand": [p.read_block(p.read32(gm + 0xa8 + 4 * i) + 0x12b0, 8)[4:].hex() for i in range(4)]},
              open(out + ".setup.json", "w"))
    o, op, om = open(out, "wb"), open(out + ".pad", "wb"), open(out + ".mark", "wb")
    oq = open(out + ".path", "wb")
    cnt = 0
    v, n = v0, 0
    while n < want:
        k = v - v0 + 1
        put(f[k] if k < len(f) else NEUTRAL)
        time.sleep(0.004)
        gm, r = regions(p)
        a = p.settle(r + [(m + 0x50, 0x160)], v)  # the marker read in the same settled frame
        if p.read32(GM_PTR) != gm: sys.exit(f"CAPTURE FAILED: match over at frame {v}, {n} samples")
        if a is None: print(f"missed frame {v} (it ticked mid-read)", flush=True)  # a gap, as record_p2m2.py
        else:
            a, mk = a[:-0x160], a[-0x160:]
            o.write(struct.pack("<I", v) + a); op.write(struct.pack("<I", v) + raw()); n += 1
            om.write(struct.pack("<I", v) + mk)
            c = struct.unpack_from("<i", mk, 0x144 - 0x50)[0]
            if c and not cnt:
                ln = struct.unpack_from("<i", mk, 4)[0]
                oq.write(struct.pack("<II", v, ln) + p.read_block(struct.unpack_from("<I", mk)[0], 0x30 * ln))
                print(f"vsync {v}: smash point placed, path {ln}", flush=True)
            cnt = c
        if n % 600 == 0: print(n, flush=True)
        w = tick(v)
        if w != v + 1: sys.exit(f"CAPTURE FAILED: missed vsync {v + 1}..{w - 1} (slow PCSX2 down)")
        v = w
    o.close(); op.close(); om.close(); oq.close()
    # the game sees frame k's bytes in the packet sampled d frames later; one d must hold for the whole recording
    pads = open(out + ".pad", "rb").read()
    got = {struct.unpack_from("<I", pads, 22 * i)[0] - v0: pads[22 * i + 4:22 * i + 22] for i in range(n)}
    for d in range(0, 4):
        bad = [i for i in got if 1 + d <= i < len(f) + d and got[i] != f[i - d]]
        if not bad: print(f"done {n}: pad bytes equal the recording on every frame (seen {d} frame(s) later)"); break
    else: sys.exit("CAPTURE FAILED: the game's pad bytes don't follow the recording")
