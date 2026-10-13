#!/usr/bin/env bash
# Quiet `cargo test` for agents: tools/check.sh [cargo test args, e.g. -p hst-sim --test flights].
# Passing: one line. Build error: the error lines only. Failing tests: their panic messages only.
# Every line cut to 300 chars, at most 60 lines. Full output: context/notes/check.log.
cd "$(dirname "$(realpath "$0")")/.."
log=context/notes/check.log
mkdir -p context/notes
# HST_CHECK_SLOTS test runs at once across main and every worktree (default 2), each capped to its share of the
# cores: five uncapped at once (each using every core) took ~30 min apiece, one at a time left cores idle
# during links and long single-threaded tests
n=${HST_CHECK_SLOTS:-2} jobs=$(( $(nproc) / ${HST_CHECK_SLOTS:-2} )); [ "$jobs" -ge 1 ] || jobs=1
lock="$(git rev-parse --git-common-dir)/check.lock"
until for i in $(seq 0 $((n - 1))); do exec 9>"$lock.$i"; flock -n 9 && break 2; done; false; do sleep 5; done
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-$jobs}
# 9>&-: cargo mustn't inherit the lock, or the sccache server it starts holds it forever
cargo test -j "$jobs" -q --message-format=short "$@" >"$log" 2>&1 9>&- && {
  echo "TESTS PASSED: $(grep -o '[0-9]* passed' "$log" | awk '{s+=$1} END{print s+0}') tests"; exit 0; }
if grep -q '^test result: FAILED' "$log"; then
  echo "TESTS FAILED (this is a test result, the build is fine):"
  awk '/^failures:$/{f++; next} f==1 && !/^note: run with/ && NF' "$log"
else
  echo "BUILD FAILED (treat this as a build failure, no tests ran):"
  grep -E '(^|: )error(\[E[0-9]+\])?:' "$log"
fi | cut -c1-300 | head -60
echo "(full log: $log)"
exit 1
