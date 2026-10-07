# python3 tools/test_ctx.py — checks lookups and that the TTL cache picks up an edit once it expires
import os, sys, tempfile, time
sys.path.insert(0, os.path.dirname(__file__))
import ctx

d = tempfile.mkdtemp()
p = os.path.join(d, 't.md')
open(p, 'w').write('# A\n- [x] **P1 — done.**\n- [ ] **P2 — open.**\n  more P2\n## B\nb1\n')
assert ctx.section(p, 'next') == ['- [ ] **P2 — open.**', '  more P2']
assert ctx.section(p, 'b') == ['## B', 'b1']
assert ctx.index(p) == ['# A', '[x] P1', '[ ] P2', '## B']
ctx.TTL = 0.2
open(p, 'w').write('# A\n## B\nb2\n'); os.utime(p, ns=(0, time.time_ns() + 10**9))
assert ctx.section(p, 'b') == ['## B', 'b1']  # inside the TTL: cached
time.sleep(0.25)
assert ctx.section(p, 'b') == ['## B', 'b2']  # expired: edit picked up
print('ok')
