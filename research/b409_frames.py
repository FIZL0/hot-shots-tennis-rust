#!/usr/bin/env python3
"""B409: a Chrome .cpuprofile's long main-thread stretches (no idle sample for over MS ms) and, for each, the hst/bevy
functions they spent the most inclusive time in: `b409_frames.py FILE [MS] [N]`."""
import json, sys, collections, re
p = json.load(open(sys.argv[1]))
lim = float(sys.argv[2]) if len(sys.argv) > 2 else 300
n = int(sys.argv[3]) if len(sys.argv) > 3 else 8
nodes = {x['id']: x for x in p['nodes']}
parent = {c: x['id'] for x in p['nodes'] for c in x.get('children', [])}
def stack(i):
    out = []
    while i in nodes:
        out.append(nodes[i]['callFrame']['functionName'] or '(anon)')
        i = parent.get(i)
    return out
t, runs, cur = 0.0, [], None
for sid, dt in zip(p['samples'], p['timeDeltas']):
    t += dt / 1000
    name = nodes[sid]['callFrame']['functionName']
    if name == '(idle)':
        if cur and cur[1] - cur[0] > lim: runs.append(cur)
        cur = None
        continue
    if not cur: cur = [t, t, collections.Counter()]
    cur[1] = t
    seen = set()
    for f in stack(sid):
        # the game's own and bevy's systems, wgpu's pipeline/shader work, GL compile calls
        if re.search(r'^hst|preload|menu_art|character::|create_render_pipeline|compile|link_program|shader|PipelineCache|block_on|spawn_local|run_until|wasm_bindgen_futures|setTimeout|requestAnimationFrame|\bSchedule\b', f) and f not in seen:
            seen.add(f); cur[2][f[:110]] += dt / 1000
if cur and cur[1] - cur[0] > lim: runs.append(cur)
for a, b, c in runs:
    print(f"{a:8.0f}–{b:8.0f} ms  ({b - a:.0f} ms)")
    for f, ms in c.most_common(n): print(f"    {ms:7.0f}  {f}")
