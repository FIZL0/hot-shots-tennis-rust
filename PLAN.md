# PLAN — Hot Shots Tennis remaster

The rules. Tasks are in `plan/TODO.md` (open) and `plan/DONE.md` (archive): read this, then **only** the task's own
file (`tools/ctx.py <ID>`) and the files listed on its line.
Don't grep the repo to find where things live — the code map below says. Re-read files rather than remember;
other sessions change them.

## Continue

"continue" = take the first open task (`tools/ctx.py next`), read `tools/ctx.py <ID>` + its journal, carry it out
completely, then hand the wrap-up to the `chore` subagent (`tools/ctx.py tick <ID>`, write the journal
entry from your notes, run `tools/check.sh`, commit with the message you give it), stop and report. One task per
iteration; don't merge or skip ahead. Too big → split it into sub-tasks (new `plan/<ID>.md` files + lines in `plan/TODO.md`) first.

- **Match the original exactly** — no approximations or placeholders. Find it in the decompile (`context/decomp/`,
  `research/fn.sh <addr>`), port it, prove it against the real game: a test compares the port to a PINE recording
  or save-state capture and passes bit-exact (f32 via `hst_sim::ps2`) or frame-exact.
- **PS2 arithmetic**: EE FPU chops, masks the smaller addend to exponent diff − 1 guard bit; `madd.s` = that add of
  acc + truncated product. Use `hst_sim::ps2` / `hst_sim::vu0`.
- **Game data from the user's disc at runtime**, never committed (`hst_data::iso`, `xb`, `exe` — only `exe` knows
  retail data offsets, disc-ID checked). Characters too (`crates/hst/src/character.rs`).
- **Same input, same result**: P0's replay harness (`hst_sim::replay`) is the acceptance test for gameplay.
- **Handedness**: read it from character data (Carol, Will lefty); mirror forehand/backhand, reach, swing/serve,
  toss hand, racket.
- **All UI at any aspect ratio** (user 2026-10-08): every HUD element, pop-up, menu, result/loading screen and
  new screen must lay out for any aspect ratio and resolution, pinned/scaled like B17's panels
  (`play/widescreen.rs`), never locked to 4:3 or one resolution. Any UI task isn't done until it is.
- **Look at the original too**: `tools/screenshot.sh` (PCSX2) vs `--shot out.png` at the same moment for anything
  visual; screenshots go in `context/`, not the journal.
- Findings → `research/journal/<date>-<slug>/`. When a task lands: tick it, add to `plan/REFERENCE.md` *Done* /
  *Play it now* if it changed.
- Game control (PINE, vpad, save-state slots 3/4/5 = user's, load only; scratch 8/9): `plan/REFERENCE.md`.

## When blocked

Stalled (tool hangs, PCSX2 won't cooperate, same fix fails twice, ~20 min on one obstacle, needs the human):
have `chore` commit what's solid (stash experiments), write details in the journal and `tools/ctx.py block <ID> <why + what unblocks>`, move to the next `- [ ]` — preferably another section. Never end a
run while `- [ ]` lines remain; at the end list blockers. Human-only items go under **Needs the human**, never into
code as guesses. Don't touch PCSX2 while `pgrep -f record_p2m2` runs.

## Tasks

Open tasks: `plan/TODO.md` (index: `tools/ctx.py`, next: `tools/ctx.py next`). Finished ones are archived in
`plan/DONE.md` (`tools/ctx.py <ID>` finds either). Never edit their checkboxes by hand: `tools/ctx.py tick <ID>` /
`tools/ctx.py block <ID> <why + what unblocks>`.

## Code map

`crates/hst-data/src/` — disc formats (no game logic)

- `iso.rs` ISO9660 reader · `xb.rs` XB archives · `tim2.rs` textures · `mdl.rs` models (+collision) · `mtl.rs`
  materials · `ani.rs` ANI2 motions · `layout.rs` court layout/plant records · `exe.rs` GAME.BIN data offsets
  (the only place with retail offsets) · `bin/` xbdump, tm2png, mdlstat
- tests: `characters.rs`, `collision.rs`

`crates/hst-gltf` — exports HST characters (all motions, faces) to `.glb` for modding (`hst-gltf hst context/xb DIR`)

`modding/` — character mod standard, tutorial, `hst_skeleton.json`; `tools/` rerig/check/render/make_skeleton (Python)

`crates/hst-sim/src/` — the game, bit-exact, no Bevy

- arithmetic: `ps2.rs` EE FPU · `vu0.rs` VU0 · `libm.rs` game sinf/acosf/atan2f · `quat.rs` matrix↔quat, slerp
- ball: `ball.rs` Flight, bounces, materials, COURTS · `contact.rs` plane sweep · `mesh.rs` triangle sweep, world
  query · `world.rs` prop placement, 20 m grid · `court.rs` court collision world from disc
- shots: `shot.rs` TRAJ tables, launch · `params.rs` per-character shot records · `swing.rs` contact search,
  timing grades · `serve.rs` walk/toss/serve contact/box aim
- players: `player.rs` locomotion, mover, turn, stamina · `motion.rs` motion state machine · `pose.rs` first-frame
  bone matrices
- match: `score.rs` score/rotation/ends · `judge.rs` line calls, verdicts (Rally) · `flow.rs` post-point timeline,
  serve placement
- camera: `camera.rs` match camera mode 0 · `cutaway.rs` post-point cut-aways
- `replay.rs` fixture reader · `bin/replay.rs` replay CLI
- tests: `ball_path flights live world` (ball) · `shot_tables swing serve` · `player motion` · `score replay` ·
  `camera cutaway`

`crates/hst/src/` — the Bevy app

- `main.rs` args, disc load, court models, orbit, auto_shot · `play.rs` (1.4k lines) the match: setup, input,
  strike/serve/locomote/advance_stroke, bot, simulate, react/next_point, camera, draw, motions, balloons, hud ·
  `character.rs` disc characters, rig, animate · `sandbox.rs`
- tests: `line_calls.rs`, `shot_params.rs`

`tools/` — `pcsx2-hst.sh` launch · `pine.py` RAM/states · `vpad.py` virtual pad · `screenshot.sh` ·
`record_p2m2.py` `record_live.py` `record_camera.py` `record_cutaway.py` `trace_live.py` `trace_shots.py`
recorders · `overnight.sh` unattended runs. `research/` — RE scripts (`fn.sh`, ghidra_scripts), journal;
`research/README.md` rebuilds `context/`.

## Needs the human

- M8 (stretch): dumps of each game, in M8's order (Hot Shots Tennis: Get a Grip PSP, Hot Shots Golf Fore! PS2, Hot Shots Golf: Open Tee 2 PSP, Hot Shots Golf: Out of Bounds PS3), to pull the characters' models, textures, stats and voice lines from.
- M5 (stretch): where the chaiNNer upscaling chain is and how to run it.
- **Record a tiebreak (for P12, P13, P3e7)**: no fixture reaches one yet. The game's sets need a 2-game lead (slot 5's match ended 5-3, not 4-3; as `score.rs`), so it only goes to a tiebreak at 4-4. Score words: games 0x42306c/0x423070 (team 0/1), sets 0x423074/78, per-set history 0x42307c + 0x14·team. Recipe (2026-10-08): on a free copy, load slot 8 (slot 5 with team 0 at 3 games), poke games and history to 4-3 (team 0 = 4), let the bots play: team 1 wins the next game → 4-4 → tiebreak; save slot 9 there, then `tools/record_p2m2.py 9 context/fixtures/tiebreak.bin 12000` and `tools/record_umpire.py 9 12000 context/fixtures/umpire_tiebreak.bin`. Three tries on copy 6 were cut short because the copy's window was closed mid-run (emulog: pause → shutdown), so whoever runs it: leave the PCSX2 window open until it's done (~15 min).
- **Doubles: who of two teammates gets the ball (for N5a)**: the port lets only one teammate hit (the first to lock a swing; the other's press whiffs, `play.rs theirs()`), from the recordings never having two teammates locked at once, but the original's code for this wasn't found. Record a doubles rally from slot 3 (P1 human + bot partner) where you and your partner both go for the same ball, pressing early and late, with `tools/record_p2m2.py 3 context/fixtures/partner_s03.bin <frames>`, so the rule (first lock, nearer player, or the human first) can be checked — j: 2026-10-07-n5a-lob-off-lob
- **Bot serve cases the bot matches never hit (for P11i)**: the singles bot match had no second serve and no serve-and-net-dash, and a bot never receives a human serve, so three parts of the AI are still unchecked. Make a save state + input recording for each (same as `bots_singles.p2m2`, saved in `context/recordings/`, note the characters and costumes):
  1. *Return of serve* (the singles receive chooser): a singles match, you (P1) serving to a bot, 10+ points across deuce and ad courts, flat, slice, lob and underhand serves, fault on purpose a few times so the bot also returns second serves → `context/recordings/human_serve_singles.p2m2`.
  2. *Second serves*: a bot-only singles match that runs until the bots fault several first serves (a long match, or easier costumes so the bots err more) → `context/recordings/bots_singles_faults.p2m2`.
  3. *Serve net dash*: a bot-only singles match with a net-rushing (serve-and-volley) character serving at the hardest costume, several of its service games → `context/recordings/bots_singles_net.p2m2`.
  Agents then replay them with `tools/record_ai_serve.py` / `record_ai_aim.py … singles` and extend `crates/hst-sim/tests/ai_serve.rs` — j: 2026-10-07-p11-ai/10-P11I-BOTS-FINAL.md
- P13 umpire: record a long match (let, deuce again, set/tiebreak/match announcements) — j: 2026-10-07-p13-umpire/2-RECORDING-READY.md
