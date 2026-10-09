# Agent notes

Rust/Bevy remaster of Hot Shots Tennis (SCUS-97610). Faithful: same AI, physics, timing; better rendering.

- Retail data is read from the user's ISO at runtime. Never commit game data, extracted assets, or dumps.
- No retail addresses or Ghidra names (`FUN_…`, `DAT_…`) in shipped code; describe behaviour instead.
  Single exception (user decision, 2026-10-06): `hst-data::exe` reads tuning *data* that only exists inside the
  game program from the user's own GAME.BIN at documented offsets, after checking the disc is SCUS-97610 US 1.00.
  No code addresses, no game data in git.
- Exactness: ported math uses `hst_sim::ps2` (PCSX2's EE FPU model: chop rounding, aligned adds) wherever the
  original's float results must match bit for bit.
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
open), finished ones in `plan/DONE.md`. "continue" means: `tools/ctx.py next`, read only `tools/ctx.py <ID>`, its
listed files and journal, port + verify, `tools/ctx.py tick <ID>` (moves it to DONE.md), commit. Don't grep the repo for where things live — the code map says. Background (game
control, done list, known gaps): `plan/REFERENCE.md`. `tools/ctx.py` (index) · `next` · `N1e` (plan/N1e.md, else
its TODO/DONE line) · `"when blocked"` (a section) · `-f FILE <name>` (any markdown) · `tick <ID>` · `block <ID> <why>`.

## Cheap context
- Decompile: `research/fn.sh <addr>` (one function), `research/xref.py <addr>` (callers/callees),
  `research/xref.py <addr> --similar` (look-alike functions; `ported` ones have Rust to copy the shape of). A hook
  blocks reading the 16 MB dumps whole.
- Tests: `tools/check.sh [-p crate --test name]`, not bare `cargo test`; full log in `context/notes/check.log`.
- `CAPTURE FAILED` from any PINE tool means the output file is incomplete: fix PCSX2, re-record.
- Hand anything mechanical to the `chore` subagent instead of doing it yourself: journal write-ups from your notes,
  ticking/blocking tasks (`tools/ctx.py tick`/`block`), checking a capture, the final test run + commit. Keep porting, the test loop,
  decompile reading and gameplay decisions.

## Controlling the game
`plan/REFERENCE.md` → *Controlling the real game* (PINE, virtual pad `tools/vpad.py`, `tools/screenshot.sh`; remaster `--shot`s in a fixed-size floating window: `tools/shot.sh`) and
`PLAN.md` → *When blocked* (switch prompts, never stop while work remains). Unattended runs: `tools/single.sh`
(one task at a time) or `tools/parallel.sh [N] [p|d] [f]` (N at once in worktrees; p = TODO order, d = different files, f = past the 90% weekly cap). Multi-step game drives
go under `tools/pcsx2.sh <cmd>`; pine.py tools take the same lock by themselves. In a parallel run `HST_PCSX2=N` gives
each agent its own PCSX2 copy (tools, pad, PINE and lock all follow it). When you're done with the game for your task,
close yours: `tools/pcsx2.sh tools/pcsx2-hst.sh stop` (waits for anything mid-use). Never `pkill pcsx2-qt`.

## If you get stuck, move on
Never wait or loop on one obstacle. If a tool hangs, PCSX2 won't cooperate, the same fix fails twice, something
needs the human, or ~20 minutes go by without progress: write what happened and what you tried in the prompt's
journal, `tools/ctx.py block <ID> <one-line reason>`, commit what's solid, and start the next open
prompt. Don't touch PCSX2 while a capture (`pgrep -f record_p2m2`) is running.
