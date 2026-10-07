# PLAN — Hot Shots Tennis remaster

The index. Read this, then **only** the task's own file (`plan/<ID>.md`) and the files listed on its line.
Don't grep the repo to find where things live — the code map below says. Re-read files rather than remember;
other sessions change them.

## Continue

"continue" = take the first `- [ ]` line under **Tasks**, read `plan/<ID>.md` + its journal, carry it out
completely, then hand the wrap-up to the `chore` subagent (tick it here and in its plan file, write the journal
entry from your notes, run `tools/check.sh`, commit with the message you give it), stop and report. One task per
iteration; don't merge or skip ahead. Too big → split it into sub-tasks (new `plan/<ID>.md` files + lines here) first.

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
- **Look at the original too**: `tools/screenshot.sh` (PCSX2) vs `--shot out.png` at the same moment for anything
  visual; screenshots go in `context/`, not the journal.
- Findings → `research/journal/<date>-<slug>/`. When a task lands: tick it, add to `plan/REFERENCE.md` *Done* /
  *Play it now* if it changed.
- Game control (PINE, vpad, save-state slots 3/4/5 = user's, load only; scratch 8/9): `plan/REFERENCE.md`.

## When blocked

Stalled (tool hangs, PCSX2 won't cooperate, same fix fails twice, ~20 min on one obstacle, needs the human):
have `chore` commit what's solid (stash experiments), write details in the journal and mark the line
`- [~] **ID** … BLOCKED: <why + what unblocks>`, move to the next `- [ ]` — preferably another section. Never end a
run while `- [ ]` lines remain; at the end list blockers. Human-only items go under **Needs the human**, never into
code as guesses. Don't touch PCSX2 while `pgrep -f record_p2m2` runs.

## Tasks

`→` files to open (play.rs fns by name) · `t:` tests · `j:` journal under `research/journal/`. Tests are in the
crate's `tests/`. Fixtures in `context/fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)
- [x] **N5** Lob smash → `hst-sim/src/swing.rs` `shot.rs` · `play.rs` find_contact/advance_stroke | t: swing.rs (fixture new_recording.bin)
- [x] **N6** Diving for the ball (P4's open dive branch) → `swing.rs` · `motion.rs` · `player.rs` · `play.rs` find_contact/advance_stroke/locomote | t: swing.rs, motion.rs, player.rs (fixture new_recording.bin)
- [x] **N2b** Ball regressed (bug, user 2026-10-07): it is tiny now. It must be big with the game's outline like it was before → `play.rs` ball draw · `character.rs` · N2's ball1.mdl/shadow swap (e06fa6a) | t: serve.rs
- [x] **F0** Uncapped fps, 60 Hz sim unchanged → `main.rs` (present mode) · `character.rs` animate/tick · `play.rs` draw/camera/balloons/hud | t: hst-sim all untouched
- [x] **N1d1** Exact ANI sampler (in N1d) → `hst-sim/src/pose.rs` · `character.rs` animate | t: motion.rs clip_sampler_ram | j: 2026-10-06-n1-animations
- [x] **P5a** Verify hit types (user 2026-10-07): the original has only ✕ normal, ○ cut, △ lob; we have flat and drive extra. Match the button set and how each is used → `play.rs` read_input/press/strike · `shot.rs` `params.rs` | t: shot_tables.rs | j: 2026-10-07-p5a-hit-types
- [x] **N1d2** Motion time/speed per frame (in N1d) → `motion.rs` `pose.rs` · `character.rs` Motion/tick · `play.rs` motions | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1d3a** Stroke arm table `0x355650` (in N1d3) → `hst-sim/src/pose.rs` | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1d3b** Contact solve `0x3561e0` body step + arm quats (in N1d3) → `pose.rs` · `swing.rs` | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1d3c** Apply IK per frame `0x34ec70` (in N1d3) → `play.rs` find_contact/advance_stroke · `character.rs` animate | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1d3d** Volley contact solve `0x357bd0` (in N1d3) → `pose.rs` | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1d4** Faces .MOR/.UVA (in N1d) → `hst-data` new format · `character.rs` | j: 2026-10-06-n1-animations
- [x] **N1e** Motion crossfade `0x140130` (in N1d) → same as N1d + `play.rs` set_motion/motions | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N1f** Full follow-through after contact (bug) → `play.rs` motions/advance_stroke/set_motion · `motion.rs` | t: motion.rs | j: 2026-10-06-n1-animations
- [x] **N2** Ball in the server's hand + game ball model → `play.rs` hold_ball/serve_turn · `hst-sim/src/serve.rs` · `character.rs` | t: serve.rs
- [x] **N2a** Serve ball bouncing before the toss (after N2) → `play.rs` hold_ball/serve_turn/motions · `serve.rs` · `motion.rs` | t: serve.rs, motion.rs
- [x] **N3a** Sound banks, note → tone, SPU pitch (in N3) → `hst-data/src/snd.rs` · `exe.rs` pitch_table | t: hst-data sound.rs | j: 2026-10-07-n3-audio
- [x] **N3b** Voice playback in the app: ADPCM, ADSR, volume/pan (in N3) → `hst-data/src/snd.rs` · new `hst/src/audio.rs` | t: hst-data sound.rs, hst audio.rs | j: 2026-10-07-n3-audio
- [x] **N3c** (split into N3c1–6) Hit, bounce, serve sounds at their frames (in N3) → `play.rs` strike/bounce/serve_turn · audio.rs | j: 2026-10-07-n3-audio
- [x] **N3c1** Positional play: bearing/distance, falloff, stereo split, bank volume (in N3c) → new `hst-sim/src/sound.rs` · `exe.rs` stereo_tables/bank_volumes · `tools/record_sound.py` | t: hst-sim sound.rs | j: 2026-10-07-n3-audio
- [x] **N3c2** Racket hit sounds at contact: program 6 keys, volumes, pitch factors (in N3c) → `play.rs` strike · audio.rs | j: 2026-10-07-n3-audio
- [x] **N3c3** Swing whoosh at the hand, 4-frame delay case (in N3c) → `play.rs` · audio.rs | j: 2026-10-07-n3-audio
- [x] **N3c4** Bounce sounds by surface, net (in N3c) → `play.rs` bounce/serve_turn · audio.rs | j: 2026-10-07-n3-audio
- [x] **N3c5** Flight whistle (lob, framed hit) following the ball (in N3c) → `sound.rs` · `play.rs` | j: 2026-10-07-n3-audio
- [x] **N3c6** Footsteps, serve bounce, rolling scrape (in N3c) → `sound.rs` · `play.rs` | j: 2026-10-07-n3-audio
- [ ] **N3d** Player voices, umpire, gallery (in N3) → audio.rs · `play.rs` | j: 2026-10-07-n3-audio
- [ ] **N3e** Music and jingles, MIDI sequencer (in N3) → `snd.rs` · audio.rs | j: 2026-10-07-n3-audio
- [ ] **N4** Landing markers as the original → `play.rs` landing/mark_landing

### Match basics
- [~] **P0c** Up to 4 players (formations, who-takes-the-ball open) → `play.rs` reset_positions/bot/read_input/control · `flow.rs` serve_placement | t: score.rs

### Shots
- [ ] **P1** Shot parameters in play → `play.rs` strike/tables · `hst-sim/src/params.rs` `shot.rs` | t: shot_tables.rs, hst/tests/shot_params.rs | j: 2026-10-05-ball-physics/5-SHOT-PARAMS-READY.md
- [ ] **P2** Aim exactly → `play.rs` aim_target/screen · `shot.rs`
- [ ] **P3** Timing grade effects → `swing.rs` · `play.rs` find_contact | t: swing.rs
- [~] **P4** All contact branches (dive, skeleton values, arm-IK step open) → `swing.rs` | t: swing.rs
- [ ] **P5** Shot selection by input → `play.rs` read_input/press/human
- [ ] **P6** Serve details, lets → `serve.rs` · `play.rs` serve_turn · `ball.rs` (bounce_turn) | t: serve.rs
- [ ] **P6a** Lob serves don't clear the net (bug, user 2026-10-07) → `serve.rs` · `play.rs` serve_turn/strike · `shot.rs` | t: serve.rs

### Players
- [ ] **P7** Exact character movement stats → `hst-sim/src/player.rs` · `play.rs` locomote/character_stats/tparam | t: player.rs
- [~] **P8** Player animation timing (overlaps N1) → `motion.rs` · `character.rs` · `play.rs` motions | t: motion.rs
- [ ] **P8a** Whiffs → `motion.rs` · `play.rs` whiff/press | t: motion.rs
- [ ] **P9** (split into P9a–f) Hit effects → new; `hst-data/src/xb.rs` `mtl.rs` (AZUMA/C_EFF)
- [x] **P9a** Effect model player (in P9) → `hst-data` mtl.rs/mor.rs · new `hst-sim/src/effect.rs` | j: 2026-10-07-p9-hit-effects
- [x] **P9b** Racket impact model (in P9) → `effect.rs` · `play.rs` strike | j: 2026-10-07-p9-hit-effects
- [x] **P9c** Hit sparks (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [ ] **P9d** Ball trail ribbon (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [ ] **P9e** Flight effects (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [ ] **P9f** Bounce and smash-bounce effects (in P9) → `effect.rs` · `play.rs` bounce | j: 2026-10-07-p9-hit-effects
- [~] **P10** Timing pop-ups (screen size/position open) → `play.rs` balloon_art/balloons/age_balloons

### AI
- [ ] **P11** Opponent AI → `play.rs` bot/bot_serve/intercept/draw_due

### Match
- [ ] **P12** Rules and scoring details → `hst-sim/src/score.rs` | t: score.rs
- [ ] **P12c** Net-cord bug: ball clips the net, drops over, and the point goes to the hitter's side. May need new recordings of net-cord rallies → `judge.rs` · `ball.rs` · `mesh.rs` (net) | t: score.rs, live.rs
- [ ] **P12a** Post-point sequence → `flow.rs` · `motion.rs` · `play.rs` react/next_point | t: score.rs, motion.rs
- [ ] **P12b** Match pop-ups → new `crates/hst/src/popups.rs` + hook in `play.rs` · `flow.rs`
- [ ] **P13** Umpire (Lily) → new; `hst-data/src/layout.rs`
- [ ] **P14** Background NPCs → `hst-data/src/layout.rs` · `main.rs` load
- [ ] **P15** Court collision mesh (mostly done by P0c3/4; check what's left) → `hst-sim/src/mesh.rs` `court.rs` `ball.rs` | t: live.rs
- [ ] **P15a** Per-court bounce profiles → `ball.rs` (COURTS) · `hst-data/src/exe.rs` · `play.rs` disc

### Presentation
- [~] **P16** Camera (court views, pick, wiring, other modes open) → `hst-sim/src/camera.rs` `cutaway.rs` · `play.rs` camera · `tools/record_camera.py` `record_cutaway.py` | t: camera.rs, cutaway.rs | j: 2026-10-06-p16-cutaway
- [ ] **P17** Court rendering fidelity → `main.rs` load/models/image · `hst-data/src/mtl.rs` `mdl.rs` `layout.rs`
- [ ] **P18** Upscaled + moddable textures → `main.rs` image · `character.rs` texture_image · `hst-data/src/tim2.rs` · `replacements/`
- [ ] **P19** HUD → `play.rs` hud/score_line
- [~] **P20** Audio (see N3) → `hst-data/src/xb.rs` | j: 2026-10-06-characters/1-MODELS-ANIM-AUDIO-PART.md
- [ ] **P21** Menus and modes → `main.rs`
- [ ] **P22** Widescreen, high fps, input polish → `main.rs` · `play.rs` read_input
- [ ] **P23** Graphics settings menu (after P21) → `main.rs`
- [ ] **P21a** Controller assignment screen (after P21): P1 picks which connected controller drives which player. The original shakes a controller's selector when that controller moves its right stick → `main.rs` · `play.rs` read_input/Pads
- [ ] **P16b** Changing ends: with more than one human player, keep the camera on the same side; only swap it in solo games (user 2026-10-07) → `play.rs` camera/next_point · `flow.rs`
- [ ] **P16a** Post-point cut-aways in play (low priority, user 2026-10-06) → `hst-sim/src/cutaway.rs` `flow.rs` `pose.rs` · `play.rs` camera/next_point · `tools/record_cutaway.py` | t: cutaway.rs | j: 2026-10-06-p16-cutaway

### Stretch
- [ ] **M1** Mod support: custom costumes for existing characters (texture swaps, model swaps), custom umpires, and fully custom characters, with a way to build them for the game logic from simple, easy-to-edit models (pick the best format) → `character.rs` · `hst-data/src/mdl.rs` `tim2.rs` · `replacements/` (P18)
- [ ] **P0b4c** Umpire-call timing (in P0b4) → `hst-sim/src/flow.rs` | t: score.rs | j: 2026-10-06-p0b-rules
- [ ] **P0b4d** Instant replay (in P0b4) → `flow.rs` · `play.rs` next_point | t: score.rs | j: 2026-10-06-p0b-rules

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

## Code map

`crates/hst-data/src/` — disc formats (no game logic)
- `iso.rs` ISO9660 reader · `xb.rs` XB archives · `tim2.rs` textures · `mdl.rs` models (+collision) · `mtl.rs`
  materials · `ani.rs` ANI2 motions · `layout.rs` court layout/plant records · `exe.rs` GAME.BIN data offsets
  (the only place with retail offsets) · `bin/` xbdump, tm2png, mdlstat
- tests: `characters.rs`, `collision.rs`

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

- (none yet)
