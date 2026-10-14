#!/usr/bin/env python3
"""B273: a Chrome .cpuprofile's self time by bucket (idle, GC, the browser's GL/WebGPU calls, wgpu, Bevy's renderer,
the rest of the wasm) and its top functions: `b273_profile.py FILE [N]`."""
import json, sys, collections, re
p = json.load(open(sys.argv[1]))
n = int(sys.argv[2]) if len(sys.argv) > 2 else 20
nodes = {x['id']: x for x in p['nodes']}
fn, bucket = collections.Counter(), collections.Counter()
for sid, dt in zip(p['samples'], p['timeDeltas']):
    f = nodes[sid]['callFrame']
    name, url = f['functionName'] or '(anon)', f['url']
    fn[name] += dt / 1000
    if name in ('(idle)', '(program)', '(garbage collector)'): b = name
    elif 'wasm' not in url and name != 'wasm-to-js': b = 'js (page + GL/WebGPU calls)'
    elif re.search(r'wgpu_hal|glow', name): b = 'wgpu_hal'
    elif re.search(r'wgpu_core|wgpu\[|wgpu_types', name): b = 'wgpu_core'
    elif re.search(r'bevy_render|bevy_pbr|bevy_core_pipeline|bevy_sprite_render|bevy_ui_render|encase', name): b = 'bevy renderer'
    else: b = 'other wasm'
    bucket[b] += dt / 1000
total = sum(bucket.values())
for b, ms in bucket.most_common():
    print(f"{ms:9.1f} ms {100 * ms / total:5.1f}%  [{b}]")
for name, ms in fn.most_common(n):
    print(f"{ms:9.1f} ms {100 * ms / total:5.1f}%  {name[:120]}")
