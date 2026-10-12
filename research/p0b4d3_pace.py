#!/usr/bin/env python3
"""P0b4d3: the instant replay's pacing in the original.
  p0b4d3_pace.py near <slot> <save>        run (fast copy) until a replay is pending (gm+0x345), save to <save>
  p0b4d3_pace.py rec <slot> <frames> <out> per vsync from <slot>: match phase gm+0x55/0x56, tick gm+0x36c,
      replay flags gm+0x344/0x345/0x32e/0x350, gm+0x334/0x338/0x33c, skip 0x2eefc8/0x2eefd0/0x2eefe8/0x2eeff0,
      slow-mo block 0x2ef090 (on, frac, start, end, ramp n/left, in-between, period, left), camera handler
      ch=gm+0xbc +0x50/0x51 mode, +0xa8.., +0xb0 +0xb4 +0xc0 +0xc4, recorder rec=gm+0xb8 counts and shot ticks."""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

GM = 0x422f80
p = Pine()
cmd = sys.argv[1]
p.load_state(int(sys.argv[2])); time.sleep(0.5 if cmd == "near" else 0.05)
if cmd == 'near':
    t = time.monotonic()
    while time.monotonic() - t < 1000:
        gm = p.read32(GM)
        if gm and p.read8(gm + 0x345):
            p.save_state(int(sys.argv[3])); print('saved at vsync', p.read32(0x1d5780), 'tick', p.read32(gm + 0x36c)); break
        time.sleep(0.02)
    else: sys.exit('no replay within 1000 s')
    sys.exit()
out = open(sys.argv[4], 'w')
last = p.read32(0x1d5780)
for n in range(int(sys.argv[3])):
    v = last = p.next_frame(last)
    gm = p.read32(GM)
    if not gm: continue
    ch, rec = p.read32(gm + 0xbc), p.read32(gm + 0xb8)
    g = p.read_regions([(gm + 0x50, 0x10), (gm + 0x320, 0x60), (0x2eefc8, 0x30), (0x2ef090, 0x30),
                        (ch + 0x50, 8), (ch + 0xa8, 0x28), (rec + 0xd0a0, 0x18), (rec + 0x10dc0, 4)])
    m, r, sk, sm, c0, c1, rc, cur = g[:0x10], g[0x10:0x70], g[0x70:0xa0], g[0xa0:0xd0], g[0xd0:0xd8], g[0xd8:0x100], g[0x100:0x118], g[0x118:0x11c]
    i = lambda b, o: struct.unpack_from('<i', b, o)[0]
    count = i(rc, 8)
    shots = p.read_regions([(rec + k * 0xd0 + 0xa0, 0x1c) for k in range(max(0, min(count, 12)))]) if count > 0 else b''
    sh = ' '.join('%d/%d/%d/%d' % (i(shots, 0x1c * k), shots[0x1c * k + 8], shots[0x1c * k + 0x14], i(shots, 0x1c * k + 0x18))
                  for k in range(len(shots) // 0x1c))
    print(v, 'ph %d nx %d tick %d' % (m[5], m[6], i(r, 0x4c)),
          'rep %d pend %d ask %d btn %d' % (r[0x24], r[0x25], r[0x0e], i(r, 0x30)),
          '334 %d 338 %d 33c %d' % (i(r, 0x14), i(r, 0x18), r[0x1c]),
          'skip %d req %d sk %d fr %d' % (i(sk, 0), i(sk, 8), sk[0x20], i(sk, 0x28)),
          'slow %d frac %.4f a %.3f b %.3f n %d left %d btw %d per %d c %d lock %d' % (
              sm[0], struct.unpack_from('<f', sm, 4)[0], struct.unpack_from('<f', sm, 8)[0], struct.unpack_from('<f', sm, 12)[0],
              i(sm, 0x10), i(sm, 0x1c), sm[0x20], i(sm, 0x24), i(sm, 0x28), sm[0x2c]),
          'cam %#x/%#x a8 %d a9 %d ac %d b0 %d b4 %d bc %d/%d c0 %d c4 %d cc %d' % (
              c0[0], c0[1], c1[0], c1[1], i(c1, 4), i(c1, 8), i(c1, 0xc), c1[0x14], c1[0x15], i(c1, 0x18), i(c1, 0x1c), i(c1, 0x24)),
          'rec %d %d %d %d %d cur %d' % (i(rc, 0), i(rc, 4), count, i(rc, 0xc), i(rc, 0x10), i(cur, 0)),
          'shots', sh, file=out, flush=True)
