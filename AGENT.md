# Agent notes

Rust/Bevy remaster of Hot Shots Tennis (SCUS-97610). Faithful: same AI, physics, timing; better rendering.

- Retail data is read from the user's ISO at runtime. Never commit game data, extracted assets, or dumps.
- No retail addresses or Ghidra names (`FUN_…`, `DAT_…`) in shipped code; describe behaviour instead.
  Single exception (user decision, 2026-10-06): `hst-data::exe` reads tuning *data* that only exists inside the
  game program from the user's own GAME.BIN at documented offsets, after checking the disc is SCUS-97610 US 1.00.
  No code addresses, no game data in git.
  Second exception (user decision, 2026-10-15): a small curated set of promo screenshots of our own renderer (website,
  README) may be committed, in `docs/screenshots/`; not texture sheets, raw captures or bulk dumps.
- Exactness: ported math uses `hst_sim::ps2` (PCSX2's EE FPU model: chop rounding, aligned adds) wherever the
  original's float results must match bit for bit.
- The PS2 libraries are ours: FIZL0/ps2-float (`../ps2-float`: EE FPU, VU0, the SDK's fdlibm, newlib rand/MT;
  `hst_sim::ps2`/`vu0`/`libm`/`rng` re-export it) and FIZL0/ps2-oracle (`../ps2-oracle`: the R5900 interpreter under
  `hst-oracle`), tag-pinned git dependencies shared with the decomp kits. Agents may fix and extend them when a port
  needs it (new op, missing function, wrong rounding): change it there with a test, tag a release, bump the tag here
  and in the kits (`crates/game-sim`, `crates/kit-oracle`), run their tests. Never copy them back in.
- Research: our own scripts and notes are in `research/` (in git: no game data, no decompiled code, no captures or
  screenshots); everything derived from the disc or the running game is in `context/` (git-ignored). Journal:
  `research/journal/YYYY-MM-DD-<slug>/`, one README per task, newest iteration file says whose move it is:
  `-FINAL` done, `-PART` agent continues, `-READY` needs the human. Put captures and images in `context/`, not
  the journal.
- Gameplay ports are verified against the real game over PINE (`tools/pine.py`, launch with `tools/pcsx2-hst.sh`).

## Research setup
`research/README.md` rebuilds `context/` from the disc (extract, overlay ELFs, Ghidra project, decompilation
dumps, save states, recordings) and lists the tool pitfalls.

## Plan
`PLAN.md` holds the rules and a code map; the ordered open tasks are in `plan/TODO.md` (each line names the files to
open), finished ones in `plan/DONE.md`. "continue" means: `tools/ctx.py next`, read `tools/ctx.py brief <ID>` (line, plan file,
journal, files), port + verify, `tools/done.sh <ID> "<msg>"` (tests, tick into DONE.md, commit). Don't grep the repo for where things live — the code map says. Background (game
control, done list, known gaps): `plan/REFERENCE.md`. `tools/ctx.py` (index) · `next` · `N1e` (plan/N1e.md, else
its TODO/DONE line) · `"when blocked"` (a section) · `-f FILE <name>` (any markdown) · `tick <ID>` · `block <ID> <why>` · `unblock <ID> [note]` · `move <ID> top|before ID|after ID|in SECTION` · `add <PREFIX> TEXT [@ WHERE]` (next free id) · `note <ID> TEXT` · `newid <PREFIX>`. Use these for reprioritising and new tasks instead of reading/editing TODO.md.

## Cheap context
- Every turn re-reads the whole conversation: pipe long output through `tail`/`grep`, read files by line range
  (`sed -n`, Read offset/limit), never dump whole logs or big files.
- Decompile: `research/fn.sh <addr>` (one function), `research/xref.py <addr>` (callers/callees),
  `research/xref.py <addr> --similar` (look-alike functions; `ported` ones have Rust to copy the shape of). A hook
  blocks reading the 16 MB dumps whole.
- Tests: `tools/check.sh [-p crate --test name]`, not bare `cargo test`; full log in `context/notes/check.log`.
  While working, run only the tests you touched (`tools/check.sh -p hst <filter>`): three check slots serve every
  agent, and a full-suite run holds one for many minutes. The full suite runs once, in `tools/done.sh`.
  Keep each test under ~10 s: share one match between checks, force the rare case (e.g. set the replay camera's
  closing-shot list) instead of playing on until it happens. Slower ones are `#[ignore]` with a doc line saying
  how and when to run them (`tools/check.sh -p hst <name> -- --ignored`). `check.sh` skips the netplay match tests
  (`play::netplay::tests::matches`) unless `HST_NETPLAY=1` or the filter names netplay/rollback/synctest: touch netplay,
  net or the snapshot → `HST_NETPLAY=1 tools/check.sh -p hst netplay`. Ignored: the full-match netplay/rollback runs,
  `synctest_through_a_change_of_ends` (snapshot/change of ends), `a_silent_peer_rejoins_and_a_silent_host_ends_it`
  (`recover.rs`), `short_code_on_a_testnet` (DHT/short codes). Touch those areas → run the matching one.
- Function oracle (`crates/hst-oracle`): for a pure function of its arguments and RAM (math, AI decisions, table
  lookups) check the port with `Ee::game()`/`Ee::from_ram(dump)` + `call(addr(name), ints, floats, budget)` over
  ≥10k generated inputs instead of recording it (names → addresses in `research/oracle.txt`; examples in its
  `tests/`; `use hst_oracle::HstEe` for those two). An unknown op panics with its address: add it to the interpreter
  (ps2-oracle, see above). Timing, frame flow, VU0 and
  rendering still need PCSX2 recordings.
- `CAPTURE FAILED` from any PINE tool means the output file is incomplete: fix PCSX2, re-record.
- Wrap-up is one call, no subagent: `tools/done.sh <ID> "<msg>"` (tests, tick, commit) or `… --blocked "<why>"`.
  In the main checkout (shared with other sessions) add `-- <paths you changed>`; it commits only those + plan/.
  A hook blocks `git add -A/./-u` and `git commit -a` there: stage and commit your own paths
  (`git commit -m … -- <paths>`). Bigger ad-hoc work: `git worktree add ../opentennis-<slug>`, merge when done.
  The `chore` subagent is only for bigger mechanical jobs (checking a capture, reformatting).

## Controlling the game
`plan/REFERENCE.md` → *Controlling the real game* (PINE, virtual pad `tools/vpad.py`, `tools/screenshot.sh`; remaster `--shot`s in a fixed-size floating window: `tools/shot.sh`) and
`PLAN.md` → *When blocked* (switch prompts, never stop while work remains). Unattended runs: `tools/single.sh`
(one task at a time) or `tools/parallel.sh [N] [p|d] [f]` (N at once in worktrees; p = TODO order (default), d = different files, f = past the 94% weekly cap; each slot builds in its own worktree `target/`, dependencies shared through sccache; don't set `CARGO_TARGET_DIR`). Multi-step game drives
go under `tools/pcsx2.sh <cmd>`; pine.py tools take the same lock by themselves. In a parallel run `HST_PCSX2=N` gives
each agent its own PCSX2 copy (tools, pad, PINE and lock all follow it). When you're done with the game for your task,
close yours: `tools/pcsx2.sh tools/pcsx2-hst.sh stop` (waits for anything mid-use). Never `pkill pcsx2-qt`.
Wine (Windows build checks): always `WINEDLLOVERRIDES="mscoree,mshtml=" WINEDEBUG=-all wine …` so a fresh prefix doesn't pop the
mono/gecko install dialogs on the user's desktop.
Never `rm` with a glob (`rm -f context/x/*`): Claude Code's rm check prompts even in bypass mode and unattended
runs sit on it until it auto-denies. Write each run to a fresh `context/<task>/runN/`, or let the tool overwrite its files.

## Sharing the main checkout
Other sessions and the runners' merges work in this same checkout at the same time. In it, commit only the paths you
changed: `git commit -m "…" -- <paths>`, never `git commit -a` or `git add -A`/`.` (they sweep up other sessions'
half-done edits). Commit as soon as a change is solid; uncommitted work can end up stashed or in someone else's
commit. `.git/MERGE_HEAD` exists → a merge is under way: wait for it, don't commit or touch the index. Anything bigger
than a quick edit while runners are going: work in your own worktree (`git worktree add ../opentennis-<slug>`) and
merge it into main when done. `tools/done.sh` (`git add -A`) is for the runners' task worktrees, not this checkout.

## If you get stuck, move on
Never wait or loop on one obstacle. If a tool hangs, PCSX2 won't cooperate, the same fix fails twice, something
needs the human, or ~20 minutes go by without progress: write what happened and what you tried in the prompt's
journal, `tools/ctx.py block <ID> <one-line reason>`, commit what's solid, and start the next open
prompt. Don't touch PCSX2 while a capture (`pgrep -f record_p2m2`) is running.
