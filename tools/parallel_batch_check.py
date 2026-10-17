#!/usr/bin/env python3
"""Check parallel.py's batched merges with git and the tests stubbed out: queued tasks land in one batch, a conflicting
one is left to merge alone, a failing batch falls back to one at a time, and nothing stays queued. Run: tools/parallel_batch_check.py"""
import os, sys, threading, time
sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
sys.argv = ['parallel.py', '3']
import parallel as P

calls, res = [], {}
P.slot, P.git, P.log = (lambda n: '/nonexistent'), (lambda *a, **k: None), (lambda m: None)


def merge_locked(batch, d):
    calls.append(list(batch))
    time.sleep(0.2)
    if 'BAD' in batch:
        return 'tests fail after the merge: x', []
    if batch == ['CONF']:
        return 'conflicts in a.rs', []
    return None, [b for b in batch if b != 'CONF']


def run(first, rest):
    go = lambda t: res.__setitem__(t, P.merge(t))
    ts = [threading.Thread(target=go, args=(first,))]
    ts[0].start()
    time.sleep(0.05)  # the first holds MERGE; the rest queue behind it
    for t in rest:
        ts.append(threading.Thread(target=go, args=(t,)))
        ts[-1].start()
        time.sleep(0.01)
    for t in ts:
        t.join()


P.merge_locked = merge_locked
run('A', ['B', 'CONF', 'C'])
assert calls == [['A'], ['B', 'CONF', 'C'], ['CONF']], calls
assert res == {'A': None, 'B': None, 'C': None, 'CONF': 'conflicts in a.rs'}, res
calls.clear(), res.clear()
run('D', ['BAD', 'E'])
assert calls == [['D'], ['BAD', 'E'], ['BAD'], ['E']], calls
assert res == {'D': None, 'BAD': 'tests fail after the merge: x', 'E': None}, res
assert P.WAITING == [] and P.LANDED == set(), (P.WAITING, P.LANDED)
print('OK')
