#!/usr/bin/env bash
# Quiet `cargo test` for agents: tools/check.sh [cargo test args, e.g. -p hst-sim --test flights].
# Passing: one line. Build error: the error lines only. Failing tests: their panic messages only.
# Every line cut to 300 chars, at most 60 lines. Full output: context/notes/check.log.
cd "$(dirname "$(realpath "$0")")/.."
log=context/notes/check.log
mkdir -p context/notes
# one test run at a time across main and every worktree: five at once (each using every core) took ~30 min apiece
exec 9>"$(git rev-parse --git-common-dir)/check.lock"
flock 9
# and all in main's target/: with a target/ per slot (parallel.sh m) each one rebuilt every dependency
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(realpath "$(git rev-parse --git-common-dir)/..")/target}"
cargo test -q --message-format=short "$@" >"$log" 2>&1 && {
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
