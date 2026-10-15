#!/usr/bin/env python3
"""B409: inclusive ms of the game's functions, wgpu pipeline/GL compile calls and the job queue in a .cpuprofile
window: `b409_window.py FILE FROM_MS TO_MS [N]` (ms from the profile's start, as `b409_frames.py` prints them)."""
import json, sys, collections, re
p = json.load(open(sys.argv[1]))
a, b = float(sys.argv[2]), float(sys.argv[3])
n = int(sys.argv[4]) if len(sys.argv) > 4 else 30
nodes = {x['id']: x for x in p['nodes']}
parent = {c: x['id'] for x in p['nodes'] for c in x.get('children', [])}
pat = re.compile(r'(hst(_data|_sim)?::[\w:]+|bevy_\w+::[\w:]+::(Schedule>::run|run_schedule)|create_render_pipeline|create_shader_module'
                 r'|compileShader|linkProgram|getShaderParameter|getProgramParameter|texImage2D|texSubImage2D|texStorage2D|bufferData'
                 r'|PipelineCache>::[\w:]+|prepare_assets|\(program\)|\(garbage collector\))')
t, tot, inc = 0.0, 0.0, collections.Counter()
for sid, dt in zip(p['samples'], p['timeDeltas']):
    t += dt / 1000
    if not a <= t <= b: continue
    tot += dt / 1000
    seen, i = set(), sid
    while i in nodes:
        f = re.sub(r'\[[0-9a-f]{16}\]', '', nodes[i]['callFrame']['functionName'] or '')
        m = pat.search(f)
        if m and m.group(1) not in seen: seen.add(m.group(1)); inc[m.group(1)] += dt / 1000
        i = parent.get(i)
print(f"{a:.0f}-{b:.0f} ms: {tot:.0f} ms sampled")
for k, v in inc.most_common(n): print(f"  {v:7.0f}  {k}")
