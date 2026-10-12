"""B93: per-vsync log of the result banner (+0x571 flag, +0x574 t, +0x578 stage) and the match phase (gm+0x55/0x56) next to the score words it reads
(server 0x42304c, winner 0x4230a8, result kind 0x4230b8), games and the scoreboard wait, over several game ends.
Writes a line whenever any of them changes.
usage: b93_banner_log.py <slot> <frames> <out.txt> [g0 g1]"""
import sys, time
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), open(sys.argv[3], "w")
if len(sys.argv) > 5:
    for t in range(2):
        g = int(sys.argv[4 + t]); p.write32(0x42306c + 4 * t, g); p.write32(0x42307c + 0x14 * t, g)
sb = p.read32(0x42d6c0); gm = p.read32(0x422f80)
last, n, prev = p.read32(0x1d5780), 0, None
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    s = (p.read8(sb + 0x571), p.read8(sb + 0x578), p.read32(sb + 0x574), p.read32(0x42304c), p.read32(0x4230a8),
         p.read32(0x4230b8), p.read32(0x42306c), p.read32(0x423070), p.read8(0x4230be), p.read8(gm + 0x55), p.read8(gm + 0x56))
    w = p.read32(sb + 0x184)
    if s != prev:
        out.write(f"{n} on={s[0]} stage={s[1]} t={s[2]} server={s[3]} winner={s[4]} kind={s[5]} games={s[6]}-{s[7]} over={s[8]} phase={s[9]}/{s[10]} vsync={v} wait={w}\n"); out.flush()
        prev = s
    if s[8]: break
print("done", n)
