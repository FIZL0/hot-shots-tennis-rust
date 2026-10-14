#!/usr/bin/env python3
"""B328: inclusive time per function in a Chrome .cpuprofile (each frame on the stack counted once per sample):
`b328_inclusive.py FILE [REGEX]`. Needs a wasm with names (wasm-bindgen --keep-debug, no wasm-opt)."""
import json,sys,collections,re
p=json.load(open(sys.argv[1])); pat=re.compile(sys.argv[2]) if len(sys.argv)>2 else None
nodes={x['id']:x for x in p['nodes']}; parent={}
for x in p['nodes']:
    for c in x.get('children',[]): parent[c]=x['id']
def short(n):
    n=re.sub(r'\[[0-9a-f]{16}\]','',n); return n[:150]
inc=collections.Counter(); tot=0
for sid,dt in zip(p['samples'],p['timeDeltas']):
    tot+=dt; seen=set(); i=sid
    while i in nodes:
        f=short(nodes[i]['callFrame']['functionName'] or '(anon)')
        if f not in seen: seen.add(f); inc[f]+=dt
        i=parent.get(i)
for f,t in inc.most_common(400):
    if pat is None or pat.search(f): print(f"{t/1000:8.0f} ms {100*t/tot:5.1f}% {f}")
