#!/usr/bin/env python3
"""B409: a web_check run's longest page frames from the boot to the match start's end (`started in`), with the start's
logged lines: `b409_start_frames.py OUT [N]` (OUT/console.txt with THROTTLE's ms stamps, OUT/frames2.json)."""
import json, sys, re
out = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 10
lines = [l for l in open(f'{out}/console.txt') if l.startswith('[2] ') and re.search(r'start|loading screen|Creating new window', l)]
for l in lines:
    print(re.sub(r'%c|color:.*', '', l).strip()[:150])
end = max((int(m.group(1)) for l in lines if 'started in' in l for m in [re.search(r'\] (\d+)ms', l)] if m), default=1e9)
f = [x for x in json.load(open(f'{out}/frames2.json')) if x[0] <= end + 2000]
print('frames to the start\'s end + 2 s:', len(f), 'longest:', sorted(f, key=lambda x: -x[1])[:n])
