# PLAN — Hot Shots Tennis remaster

The rules. Tasks are in `plan/TODO.md` (open) and `plan/DONE.md` (archive): read this, then **only** the task's own
file (`tools/ctx.py <ID>`) and the files listed on its line.
Don't grep the repo to find where things live — the code map below says. Re-read files rather than remember;
other sessions change them.

## Continue

"continue" = take the first open task (`tools/ctx.py next`), read `tools/ctx.py brief <ID>` (its line, plan file,
journal and files in one go; the runners paste it into your prompt already), carry it out completely, write the journal
entry, then `tools/done.sh <ID> "<commit message>"` (tests → tick → commit in one call), stop and report. One task per
iteration; don't merge or skip ahead. Too big → split it into sub-tasks (new `plan/<ID>.md` files + lines in `plan/TODO.md`) first.

- **Match the original exactly** — no approximations or placeholders. Find it in the decompile (`context/decomp/`,
  `research/fn.sh <addr>`), port it, prove it against the real game: a test compares the port to a PINE recording
  or save-state capture and passes bit-exact (f32 via `hst_sim::ps2`) or frame-exact.
  This covers gameplay only: physics, AI, shots, scoring, rules and match timing.
- **Rendering only has to look right** (user 2026-10-12): shadows, shade maps, lighting, effects and GS output
  match the original by eye or within a small tolerance against a capture, not bit for bit. Don't replay GS/VU1
  rounding to close the last few pixels, and don't open a follow-up "(X gap)" task just to chase remaining
  bit-level render differences: note them in the journal and move on.
- **The main menu and front-end don't have to match the original** (user 2026-10-12): title, mode menus, character/
  court/umpire select, settings, loading and result screens are ours to design. The original is a reference for
  them, not a spec; no bit-/frame-exact proof needed. Where a task in `plan/TODO.md` says a menu must look or behave
  "like the original" / "1:1", treat that as a starting point, not a requirement.
- **In-match presentation only has to feel right** (user 2026-10-12): instant replays (when they trigger and how
  they play), cameras, cutaways, crowd/umpire animation, commentary and SFX cues match the original by eye. No oracle
  or PINE proof; just check it doesn't touch the sim RNG or match timing. Oracle runs stay at ≥10k inputs; no
  mutation testing unless a test looks too weak.
- **Additions are layered on top** (character/texture mods, other games'
  characters, widescreen, upscaled textures): with them off or unused, a match must still play
  exactly like the original (the replay tests stay bit-exact). A mod character uses the same physics and AI code
  paths as a stock one, fed its own data.
- **PS2 arithmetic**: EE FPU chops, masks the smaller addend to exponent diff − 1 guard bit; `madd.s` = that add of
  acc + truncated product. Use `hst_sim::ps2` / `hst_sim::vu0`.
- **Game data from the user's disc at runtime**, never committed (`hst_data::iso`, `xb`, `exe` — only `exe` knows
  retail data offsets, disc-ID checked). Characters too (`crates/hst-base/src/character.rs`).
- **Same input, same result**: P0's replay harness (`hst_sim::replay`) is the acceptance test for gameplay.
- **Handedness**: read it from character data (Carol, Will lefty); mirror forehand/backhand, reach, swing/serve,
  toss hand, racket.
- **All UI at any aspect ratio** (user 2026-10-08): every HUD element, pop-up, menu, result/loading screen and
  new screen must lay out for any aspect ratio and resolution, pinned/scaled like B17's panels
  (`hst-base/src/widescreen.rs`), never locked to 4:3 or one resolution. Scale uniformly, never stretch (user 2026-10-13): an element keeps
  its own aspect at any window size; extra space goes around it. Any UI task isn't done until it is.
- **Look at the original too**: `tools/screenshot.sh` (PCSX2) vs `--shot out.png` at the same moment for anything
  visual; screenshots go in `context/`, not the journal.
- Findings → `research/journal/<date>-<slug>/`. When a task lands: tick it, add to `plan/REFERENCE.md` *Done* /
  *Play it now* if it changed.
- Game control (PINE, vpad, save-state slots 3/4/5 = user's, load only; scratch 8/9): `plan/REFERENCE.md`.

## When blocked

Stalled (tool hangs, PCSX2 won't cooperate, same fix fails twice, ~20 min on one obstacle, needs the human):
stash experiments, write details in the journal and `tools/done.sh <ID> "<msg>" --blocked "<why + what unblocks>"`.
PCSX2 runs unattended (`tools/pcsx2-hst.sh`, `tools/screenshot.sh`, PINE): "couldn't capture / not launched" is not a
blocker, launch it; move to the next `- [ ]` — preferably another section. Never end a
run while `- [ ]` lines remain; at the end list blockers. Human-only items go in `plan/HUMAN.md`, never into
code as guesses. Don't touch PCSX2 while `pgrep -f record_p2m2` runs.

## Tasks

Open tasks: `plan/TODO.md` (index: `tools/ctx.py`, next: `tools/ctx.py next`). Finished ones are archived in
`plan/DONE.md` (`tools/ctx.py <ID>` finds either). Never edit their checkboxes by hand: `tools/ctx.py tick <ID>` /
`tools/ctx.py block <ID> <why + what unblocks>` / `tools/ctx.py unblock <ID> [note]`.

## Code map

`crates/hst-data/src/` — disc formats (no game logic)

- `iso.rs` ISO9660 reader · `xb.rs` XB archives · `tim2.rs` textures · `mdl.rs` models (+collision) · `mtl.rs`
  materials · `ani.rs` ANI2 motions · `layout.rs` court layout/plant records · `exe.rs` GAME.BIN data offsets
  (the only place with retail offsets) · `bin/` xbdump, tm2png, mdlstat
- tests: `characters.rs`, `collision.rs`

`crates/hst-gltf` — exports HST's own characters (all motions, faces) to `.glb` as modding reference rigs (`hst-gltf hst context/xb DIR`); other games' extraction and mod packaging live in ../opentennis-mods

README *Character mods* — links the mods repo (../opentennis-mods: standard, tutorial, extract/rerig tools); `crates/hst/hst_skeleton.json` is the game's copy of its skeleton

`crates/hst-sim/src/` — the game, bit-exact, no Bevy

- arithmetic: `ps2.rs` EE FPU · `vu0.rs` VU0 · `libm.rs` game sinf/acosf/atan2f (all three re-export our
  `ps2-float` crate, ../ps2-float) · `quat.rs` matrix↔quat, slerp · `rng.rs` the match's four generators on
  ps2-float's newlib `rand()` + MT19937
- ball: `ball.rs` Flight, bounces, materials, COURTS · `contact.rs` plane sweep · `mesh.rs` triangle sweep, world
  query · `world.rs` prop placement, 20 m grid · `court.rs` court collision world from disc
- shots: `shot.rs` TRAJ tables, launch · `params.rs` per-character shot records · `swing.rs` contact search,
  timing grades · `serve.rs` walk/toss/serve contact/box aim
- players: `player.rs` locomotion, mover, turn, stamina · `motion.rs` motion state machine · `pose.rs` first-frame
  bone matrices
- match: `score.rs` score/rotation/ends · `judge.rs` line calls, verdicts (Rally) · `flow.rs` post-point timeline,
  serve placement
- camera: `camera.rs` match camera mode 0 · `cutaway.rs` post-point cut-aways
- `replay.rs` fixture reader · `bin/replay.rs` replay CLI · `bin/state_hash.rs` per-tick hash, same on x86_64/aarch64/wasm32 (`tests/platforms.rs`); transcendentals only via `libm` (game's) or `musl`, never std
- tests: `ball_path flights live world` (ball) · `shot_tables swing serve` · `player motion` · `score replay` ·
  `camera cutaway` · `line_calls shot_params`

`crates/hst-oracle` — test-only function oracle: `hst.rs` (disc check, `HstEe::game`/`from_ram`, `addr`) on our
`ps2-oracle` interpreter (../ps2-oracle); tests compare ports to the game's own functions

`crates/hst-base/src/` — the Bevy app's lower layer (P60: its own crate, so an edit in the match or menus rebuilds
and relinks only `hst`); `hst`'s `main.rs` re-exports it (`pub use hst_base::*`), so `crate::audio` etc. still resolve
there. It never names `hst`: hooks the top fills in go through statics/generics (`graphics::SAVE`, `graphics::plugin::<Game>`)

- `lib.rs` `Args`, `Orbit`, `store` (settings file I/O), `Wire` (snapshot encoding) · `handshake.rs` online match seed
  (`MatchSeed`) and peer handshake check (M2e) · `mods_online.rs` mods in online lobbies: gameplay wire, looks packs,
  stand-ins (M2i) · `mods_umpire.rs` umpire mods: manifest, calls over the donor's bank (M8f3) · `mods_textures.rs` texture packs: manifest,
  order/off (M5a) · `mods.rs`/`mods/`
  `modfs.rs` `modstore.rs` `mods_court.rs` `court_mod.rs` character/court mods, zips, store · `character.rs` disc
  characters, rig, animate · `net.rs`/`net/` iroh transport, lobbies, codes · `audio.rs` · `graphics.rs` settings,
  auto-tune · `gs.rs` `shade.rs` `shadow.rs` `textures.rs` `lod.rs` `cull.rs` rendering · `widescreen.rs` HUD
  pinning (B17) · `seats.rs` pads → seats · `ui_pool.rs` overlay quad pools only while shown (P39) · `ui_census.rs`
  `HST_UI_CENSUS=1` UI node counts/changes per root · `web.rs` `ios.rs` `android.rs` platform glue
- test helpers `hst`'s tests use are `#[cfg(any(test, feature = "testing"))]` (hst's dev-dependency turns `testing` on)

`crates/hst-play/src/` — the match (P61: its own crate under the menus, so a menu edit rebuilds only `hst` and a
match edit is checked with hst-play's own tests); `hst`'s `main.rs` re-exports it (`pub use hst_play::*`) and splices
the front-end into its `play` module (`mod play { pub use hst_play::play::*; pub mod main_menu; pub mod menu; }`), so
`crate::play::…` paths still resolve there. It never names the menus: the pause state and preview faces come through
`play::front_end::FrontEnd`, a resource `play/menu.rs` fills in

- `lib.rs` · `play.rs` the match: setup, input, strike/serve/locomote/advance_stroke, bot, simulate, react/next_point,
  camera, draw, motions, balloons, hud · `play/` match parts, netplay (+ `OnlineNotice`, `CHOICES`)/rollback/snapshot,
  `word_label.rs` word.tm2 labels (`FONT`), `legend.rs` the key legend's types and drawing, `front_end.rs` the hooks
  above · `mode.rs` mode switches/teardown · `preload.rs` · `stage.rs` court models/layout/sky (from main.rs)

`crates/hst/src/` — the game on top: menus, modes, startup

- `main.rs` startup, disc load, auto_shot, wasm entry `run` · `play/main_menu.rs` + `play/main_menu/` the front-end
  (menu rows of the legend in `main_menu/legend.rs`) · `play/menu.rs` the in-match pause screen (embeds the options
  pages) · `loading.rs` · `sandbox.rs` · `mash.rs` · `update.rs` self-updater
- no `tests/` dir on purpose (in either crate): any file there makes `cargo test -p <crate>` build the whole game a
  second time (as a binary for it); sim-only tests go in hst-sim

`tools/` — `pcsx2-hst.sh` launch · `pine.py` RAM/states · `vpad.py` virtual pad · `screenshot.sh` ·
`record_p2m2.py` `record_live.py` `record_camera.py` `record_cutaway.py` `trace_live.py` `trace_shots.py`
recorders · `overnight.sh` unattended runs. `research/` — RE scripts (`fn.sh`, ghidra_scripts), journal;
`research/README.md` rebuilds `context/`.
