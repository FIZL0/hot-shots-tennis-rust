#!/usr/bin/env python3
"""tools/trace_agg.py TRACE.json [SKIP_S] [N] — per-frame cost of every Bevy span in a trace_chrome trace.

Spans that start before SKIP_S seconds (default 9: past loading) are left out. Prints µs per frame (frames =
`main app` spans), inclusive and self, and calls per frame; top N (default 60) by self time.
"""
import collections, re, sys

rx = re.compile(r'"name":"((?:[^"\\]|\\.)*)","ph":"([BE])","pid":\d+,"tid":(\d+),"ts":([\d.]+)')
path, skip, top = sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 9.0, int(sys.argv[3]) if len(sys.argv) > 3 else 60
stacks, total, own, calls = collections.defaultdict(list), collections.Counter(), collections.Counter(), collections.Counter()
for line in open(path):
    m = rx.search(line)
    if not m:
        continue
    name, ph, tid, ts = m.group(1).replace('\\"', ''), m.group(2), m.group(3), float(m.group(4))
    st = stacks[tid]
    if ph == 'B':
        st.append([name, ts, 0.0])
    elif st:
        n, t0, child = st.pop()
        d = ts - t0
        if t0 >= skip * 1e6:
            total[n] += d; own[n] += d - child; calls[n] += 1
        if st:
            st[-1][2] += d
frames = calls['main app: '] or 1
print(f'{frames} frames; µs/frame: self, inclusive, calls/frame')
for n, v in own.most_common(top):
    print(f'{v / frames:8.1f} {total[n] / frames:8.1f} {calls[n] / frames:6.1f}  {n[:140]}')
