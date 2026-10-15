#!/usr/bin/env python3
"""B357: what the slow frames of a browser match spend their time on. `b357_spikes.py FILE.cpuprofile [N] [ROOT]`:
cuts the Chrome CPU profile into frames (runs of consecutive samples whose stack holds ROOT, default `App>::update`),
prints the frame lengths' spread, then for the N slowest frames (default 12) the functions that take the most
inclusive time in them beyond a typical frame's (the median frame's share). Needs a wasm with names (wasm-bindgen
--keep-debug, no wasm-opt)."""
import json, sys, collections, re, statistics

p = json.load(open(sys.argv[1]))
top = int(sys.argv[2]) if len(sys.argv) > 2 else 12
root = sys.argv[3] if len(sys.argv) > 3 else 'App>::update'
nodes = {x['id']: x for x in p['nodes']}
parent = {c: x['id'] for x in p['nodes'] for c in x.get('children', [])}


def short(n):
    return re.sub(r'\[[0-9a-f]{16}\]', '', n)[:140]


stacks = {}


def stack(i):
    if i not in stacks:
        s, j = [], i
        while j in nodes:
            s.append(short(nodes[j]['callFrame']['functionName'] or '(anon)'))
            j = parent.get(j)
        stacks[i] = s
    return stacks[i]


# frames: [start µs, samples [(stack, dt)]]
frames, cur, t = [], None, 0
for sid, dt in zip(p['samples'], p['timeDeltas']):
    t += dt
    s = stack(sid)
    if any(root in f for f in s):
        if cur is None:
            cur = [t, []]
            frames.append(cur)
        cur[1].append((s, dt))
    else:
        cur = None
lens = [sum(dt for _, dt in f[1]) / 1000 for f in frames]
if not lens:
    sys.exit(f'no samples under {root}')
q = statistics.quantiles(lens, n=100)
print(f'{len(lens)} frames: median {statistics.median(lens):.1f} ms, p90 {q[89]:.1f}, p99 {q[98]:.1f}, max {max(lens):.1f}')


def inclusive(fr):
    inc = collections.Counter()
    for s, dt in fr[1]:
        for f in set(s):
            inc[f] += dt / 1000
    return inc


med = statistics.median(lens)
typical = collections.Counter()
normal = [f for f, l in zip(frames, lens) if l <= med * 1.2]
for f in normal:
    typical.update(inclusive(f))
for k in typical:
    typical[k] /= len(normal)
for f, l in sorted(zip(frames, lens), key=lambda x: -x[1])[:top]:
    inc = inclusive(f)
    extra = sorted(((v - typical.get(k, 0), k) for k, v in inc.items()), reverse=True)
    print(f'\n{l:.1f} ms at {f[0] / 1e6:.2f} s:')
    shown = 0
    for v, k in extra:
        # the frame's own scaffolding (winit, App::update, the schedule runner) spans most of every frame
        if typical.get(k, 0) > 0.6 * med:
            continue
        if v < 1 or shown >= 14:
            break
        print(f'  +{v:6.1f} ms  {k}')
        shown += 1
