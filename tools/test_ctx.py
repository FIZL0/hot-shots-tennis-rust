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
# tick / block / archive
todo, done = os.path.join(d, 'todo.md'), os.path.join(d, 'done.md')
open(todo, 'w').write('# T\n\n### S\n\n- [ ] **A1** one\n  more A1\n- [x] **A2** two\n- [ ] **A3** three\n')
open(done, 'w').write('# Done\n\n### S\n\n- [x] **A0** zero\n')
own = os.path.join(d, 'A1.md'); open(own, 'w').write('- [ ] **A1** one\n')
ctx.tick(todo, done, 'A1', d)
assert open(todo).read() == '# T\n\n### S\n\n- [ ] **A3** three\n', open(todo).read()
assert open(done).read() == '# Done\n\n### S\n\n- [x] **A0** zero\n- [x] **A1** one\n  more A1\n- [x] **A2** two\n'
assert open(own).read() == '- [x] **A1** one\n'
ctx.block(todo, 'A3', 'no tiebreak')
assert ctx.section(todo, 'A3') == ['- [~] **A3** three BLOCKED: no tiebreak']
print('ok')
