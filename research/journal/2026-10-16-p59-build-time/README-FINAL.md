# P59 — build time, one landing

Main checkout, its own `target/`, mold linker (`.cargo/config.toml`), 16 cores, no `-j` cap. Load: other slots
building, 1-min load average 3–12 during the runs; each A/B pair was run back to back at the same load (numbers in
`context/P59/`).

## Measured

`cargo test -p hst --no-run`, hst's test binary:

| | binary | link alone (mold, 5 runs, same load) | touch `main.rs` → rebuilt | real edit in `main()` |
|---|---|---|---|---|
| before | 772 MB (~600 MB debug sections) | 385–500 ms | 3.6–3.9 s | 3.9 or ~19 s |
| split-debuginfo + deps `debug = false` | 205 MB | 263–345 ms | 3.3–3.5 s | 3.5 or ~19 s |
| + bevy `dynamic_linking` (trial) | 98 MB | 90 ms | 3.2–3.3 s | — |

- Link alone: the exact link line (`cargo rustc … -- --print link-args -C save-temps`) re-run by hand, so rustc
  isn't in the number. With mold the link was never the big cost: rustc's own incremental work on hst is. A real
  edit takes either ~4 s or ~19 s depending on how much hst rustc redoes, the same under both profiles.
- split-debuginfo alone: 401 MB (the line tables stay in the binary by design); deps without debug info: 205 MB.
  That's also ~570 MB less per stale test binary left in `target/` (check.sh's 120 GB cap).
- Full dependency rebuild after the profile change: ~3 min at load ~10 (each slot does this once, sccache helps).

## Kept

1. `split-debuginfo = "unpacked"` in `profile.dev`, `debug = false` for `[profile.dev.package."*"]`; ps2-float and
   ps2-oracle (git deps, ours) keep `line-tables-only` explicitly. Our workspace crates keep file:line.
2. **Dropped** bevy `dynamic_linking`: link 265 → 90 ms, a whole cycle ~0.25 s quicker (3.45 → 3.2 s), but a
   check.sh-only feature means a second full Bevy build per slot beside `cargo build --release`/`run`'s (119 s at
   load 11, again after every Cargo change); it'd take hundreds of test builds per slot to pay back. Not added;
   tools/check.sh unchanged.
3. build.rs: HST_COMMIT and the `.git/HEAD`/`.git/logs/HEAD` watch only for `PROFILE=release` (every shipped build
   — tools/build.sh, release.sh, web.sh, apk.sh, ios-build.sh, CI — and tools/shot.sh's binary are release).
   Dev/test builds say `dev` and no longer rebuild hst on another session's commit: test build after a commit
   0.6 s (was a full hst rebuild, 4–19 s). Release still re-stamps (`Dirty hst … .git/logs/HEAD has changed`).

## Checks

`tools/check.sh -p hst-sim replay` (4 passed, bit-exact), `-p hst-sim --test platforms` (state_hash, 1 passed),
`HST_NETPLAY=1 tools/check.sh -p hst netplay` (26 passed). Opt-levels are unchanged; debug info doesn't change
codegen.

## Not verified / not 1:1

- Gameplay untouched (build settings only), so no PCSX2 check: nothing to compare.
- Timings are on a shared machine with other slots building; the per-run noise (±0.3 s, and the 4 s / 19 s modes of a
  real edit) is larger than the link saving, so the cycle numbers are medians of 5, the link numbers isolate it.
- A dev (non-release) `cargo run` now shows `dev` instead of the commit in the main menu's version corner.
