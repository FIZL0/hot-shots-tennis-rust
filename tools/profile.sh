#!/bin/bash
# tools/profile.sh OUT [SECONDS] [hst args…] — per-system CPU profile of a 4-bot match (P35): builds hst with Bevy's
# trace_chrome into target/trace, runs tools/bench.sh with it, writes OUT (tools/trace_agg.py's table: µs per frame for
# every system, schedule and render pass) and deletes the multi-GB trace. Tracing roughly halves the frame rate, so
# compare runs with each other, not with bench.sh's fps.
set -e
cd "$(dirname "$0")/.."
out=$(realpath -m "$1"); secs=${2:-15}; shift; [ $# -gt 0 ] && shift
cargo build --release -p hst --features bevy/trace_chrome --target-dir target/trace 2>&1 | grep -E "^error|Finished" >&2
trace="$out.trace.json"
HST_BIN=target/trace/release/hst HST_ENV="${HST_ENV:-} TRACE_CHROME=$trace" tools/bench.sh "$out.bench.log" "$secs" "$@"
python3 tools/trace_agg.py "$trace" 9 80 > "$out"
rm -f "$trace"
head -30 "$out"
