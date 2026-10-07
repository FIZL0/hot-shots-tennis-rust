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

- [x] **P6a** Serve ball logic (bug, user 2026-10-07): lob serves don't clear the net, and sweet-spot serves don't fly the way the original's do → `serve.rs` · `play.rs` serve_turn/strike · `shot.rs` | t: serve.rs
- [x] **P6b** Serve depth error from contact height (P6a's open part): at the swing the game scores how far the ball is from the ideal contact height (overhand or underhand ideal per character), counts it in steps past a threshold, ×6 for serves plus a bias, and adds /10 of it to the serve's depth scatter (a strong toss's short miss scaled ×3). The same pass picks `dw1` for a weak toss, a stat scaling the selector before its −0.2 threshold. Needs: the decompile of the swing setup (35b030) and serve launch (35b640/35cd50); the sources of the ideal heights, threshold, step and bias (player +0x13d0/+0x13dc/+0x1364/+0x1368, +0x3f98) and the stat; port into `serve::target`'s scatter; check against each serve's +0x3ecc/+0x3ed8 and scatter in match_s05 → `serve.rs` · `play.rs` serve_turn | t: serve.rs | j: 2026-10-07-p6a-serve-ball
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
- [x] **N3d** Player voices, umpire, gallery (in N3) → audio.rs · `play.rs` | j: 2026-10-07-n3-audio
- [x] **N3e** Music and jingles, MIDI sequencer (in N3) → `snd.rs` · audio.rs | j: 2026-10-07-n3-audio
- [x] **N4** Landing markers as the original → `effects.rs` LandingMarks · `play.rs` smash_frame/strike | j: 2026-10-07-n4-landing-markers
- [x] **N4a** Yellow smash marker on screen vs the original: get an opponent's high ball onto a human (vpad-driven rally from slot 3/4), screenshot (F8 via `hyprctl dispatch 'hl.dsp.send_shortcut(...)'`, see N4 journal) and compare frames/position with the port → `play.rs` smash_frame | j: 2026-10-07-n4-landing-markers
- [x] **P16b** Changing ends: with more than one human player, keep the camera on the same side; only swap it in solo games (user 2026-10-07) → `play.rs` camera/next_point · `flow.rs`
- [ ] **N3f** Whiff voice (bug, user 2026-10-07): the miss voice line plays on every swing that doesn't reach the ball; the original only voices some whiffs. Find its rule (chance, cooldown, which swings) and match it → `sound.rs` · `play.rs` whiff | t: sound.rs | j: 2026-10-07-n3-audio
- [ ] **N3g** Change-ends music (bug, user 2026-10-07): no music plays while the players change sides; the original plays its change-ends tune. Play it when and as the original does. N3e (music, jingles; branch task/N3e, done but not merged: conflict) plays BGM only with `--music` and has no change-ends cue; check what the original plays then → audio.rs · `play.rs` ChangeEnds | j: 2026-10-07-n3-audio
- [x] **B1** Render errors (bug, user 2026-10-07): the app floods the log with `ERROR bevy_render::slab_allocator: Use-after-free: attempted to copy element data for an unallocated key`. Find which meshes are freed while still queued (likely something spawned/despawned or mesh-replaced every frame; P9's effects came in just before) and fix so no errors print in play → `play.rs` · `character.rs` · `main.rs` — fixed: a 0-vertex dynamic effect mesh (sparks, trails, flight, bounce) gets one invisible degenerate triangle (`effects.rs` fill); Bevy 0.19 skips allocating an empty mesh but still copies it
- [ ] **N4b** Missing lob indicator (bug, user 2026-10-07): the lob landing marker sometimes doesn't show where the original shows one. Find which lobs lose it and match the original's rule → `effects.rs` LandingMarks · `play.rs` strike | j: 2026-10-07-n4-landing-markers
- [ ] **N7** Racket transparency (bug, user 2026-10-07): rackets are drawn opaque where the original's are see-through. Match its blending for the racket model → `character.rs` · `hst-data/src/mtl.rs` `tim2.rs` · `main.rs`
- [ ] **B2** Carol's net sweet spots go out (bug, user 2026-10-07): close to the net, Carol's sweet-spot hits aimed straight back land out where they probably shouldn't. Reproduce near the net with Carol (vpad, sweet-spot timing, no stick), compare the original's aim/landing for the same contact (PINE capture) and fix the target or launch → `shot.rs` `swing.rs` · `play.rs` strike
- [ ] **B3** P1 Carol's walk after a point (bug, user 2026-10-07): when P1 is Carol and she ends the point at the edge of the camera, her walk cycle back to position slides her the wrong way. Compare the original's walk-back (direction, motion, facing) from that spot and fix → `player.rs` · `motion.rs` · `play.rs` next_point/locomote
- [ ] **B4** Red marker on the serve (bug, user 2026-10-07): the red landing marker (`chakudan_p`) shows on a serve; the original never shows it for the serve, only from the return on. `strike` sets it only when `shots > 1` and `next_point` clears it, and a 2-minute all-CPU match (`HST_AUTOPLAY`) never had it up in the serve phase or on shot 1 (faults included), so reproduce the user's case first: P1 human serving (vpad), second serves, change of ends, the previous point's marker. Then match the original → `play.rs` strike/next_point/start_effects · `effects.rs` LandingMarks | j: 2026-10-07-n4-landing-markers

### Match basics

- [~] **P0c** Up to 4 players (formations, who-takes-the-ball open) → `play.rs` reset_positions/bot/read_input/control · `flow.rs` serve_placement | t: score.rs

### Shots

- [x] **N5a** Hitting a lob off a lob isn't accurate (bug, user 2026-10-07): mid-rally, returning the opponent's lob smash, the contact search gave an illegal hit point, though anyone should be able to hit that ball. Record such a rally and match the contact search and lob response to it → `swing.rs` find_contact · `shot.rs` · `play.rs` find_contact/advance_stroke | t: swing.rs | j: 2026-10-07-n5a-lob-off-lob

### Players

- [ ] **P7** (split into P7a–f) Exact character movement stats → `hst-sim/src/player.rs` · `play.rs` locomote/character_stats/tparam | t: player.rs
- [x] **P7a** Human pad → run direction, Carol/Kaito replay (in P7) → `player.rs` pad_dir · `play.rs` pad_run/human | t: player.rs | j: 2026-10-07-p7-movement
- [ ] **P7b** All 14 characters' running, own bodies; hand source (in P7) → `tests/player.rs` · `play.rs` character_hand | t: player.rs
- [ ] **P7c** Reach and contact height windows (in P7) → `player.rs` · `play.rs` character_stats | t: player.rs
- [ ] **P7d** Serve position and movement around the serve (in P7) → `player.rs` · `play.rs` serve_turn | t: player.rs
- [ ] **P7e** Dive distance, recovery, split-step (in P7) → `player.rs` · `play.rs` | t: player.rs
- [ ] **P7f** Auto-positioning and bot direction (in P7) → `player.rs` · `play.rs` bot | t: player.rs
- [~] **P8** Player animation timing (overlaps N1) → `motion.rs` · `character.rs` · `play.rs` motions | t: motion.rs
- [x] **P8a** Whiffs → `motion.rs` · `play.rs` whiff/press | t: motion.rs
- [x] **P9** (split into P9a–f) Hit effects → new; `hst-data/src/xb.rs` `mtl.rs` (AZUMA/C_EFF)
- [x] **P9a** Effect model player (in P9) → `hst-data` mtl.rs/mor.rs · new `hst-sim/src/effect.rs` | j: 2026-10-07-p9-hit-effects
- [x] **P9b** Racket impact model (in P9) → `effect.rs` · `play.rs` strike | j: 2026-10-07-p9-hit-effects
- [x] **P9c** Hit sparks (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [x] **P9d** Ball trail ribbon (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [x] **P9e** Flight effects (in P9) → `effect.rs` · `play.rs` | j: 2026-10-07-p9-hit-effects
- [x] **P9f** Bounce and smash-bounce effects (in P9) → `effect.rs` · `play.rs` bounce | j: 2026-10-07-p9-hit-effects
- [~] **P10** Timing pop-ups (screen size/position open) → `play.rs` balloon_art/balloons/age_balloons

### AI

- [ ] **P11** Opponent AI → `play.rs` bot/bot_serve/intercept/draw_due
- [x] **P11a** AIParam table and row choice (in P11) → `hst-sim/src/ai.rs` · `play.rs` setup/bot | t: ai.rs | j: 2026-10-07-p11-ai
- [ ] **P11b** AI object and update dispatch (in P11) → `hst-sim/src/ai.rs` · `play.rs` bot
- [ ] **P11c** Reaction delay (in P11) → `ai.rs` · `play.rs` bot
- [ ] **P11d** Timing error (in P11) → `ai.rs` · `play.rs` bot/bot_serve
- [ ] **P11e** Positioning (in P11) → `ai.rs` · `play.rs` bot/intercept
- [ ] **P11f** Target choice (in P11) → `ai.rs` · `play.rs` bot
- [ ] **P11g** Shot and serve type choice (in P11) → `ai.rs` · `play.rs` bot/bot_serve
- [ ] **P11h** Dive and verification (in P11) → `ai.rs` · `play.rs` draw_due | t: slot 5

### Match

- [~] **P12** Rules and scoring details → `hst-sim/src/score.rs` | t: score.rs — BLOCKED: no recorded tiebreak (fixtures cover deuce/adv, side changes, sets, doubles only); needs the human's tiebreak recording, see *Needs the human*
- [ ] **P12a** Post-point sequence → `flow.rs` · `motion.rs` · `play.rs` react/next_point | t: score.rs, motion.rs
- [ ] **P12b** Match pop-ups → new `crates/hst/src/popups.rs` + hook in `play.rs` · `flow.rs`
- [x] **P13** Umpire (Lily) → new; `hst-data/src/layout.rs`
- [ ] **P14** Background NPCs → `hst-data/src/layout.rs` · `main.rs` load
- [x] **P14a** NPC roster and placement → `hst-sim/src/npc.rs` · `hst-data/src/exe.rs` · `layout.rs` · `main.rs` load | t: npc.rs
- [x] **P14b** Walking spectators (wander/react, animation) → `hst-sim/src/npc.rs` | t: npc.rs
- [ ] **P14c** (split into P14c1–5) Trigger creatures (types, triggers, paths) → `hst-sim/src/npc.rs` · `hst-data/src/exe.rs` | t: npc.rs
- [x] **P14c2** Ambient sound emitters (timers, RNG, positions) → `hst-sim/src/npc.rs` | t: npc.rs
- [x] **P15a** Per-court bounce profiles → `ball.rs` (COURTS) · `hst-data/src/exe.rs` · `play.rs` disc

### Presentation

- [~] **P16** Camera (court views, pick, wiring, other modes open) → `hst-sim/src/camera.rs` `cutaway.rs` · `play.rs` camera · `tools/record_camera.py` `record_cutaway.py` | t: camera.rs, cutaway.rs | j: 2026-10-06-p16-cutaway
- [ ] **P17** (split into P17a–h) Court rendering fidelity → `main.rs` load/models/image · `hst-data/src/mtl.rs` `mdl.rs` `layout.rs`
- [x] **P17a** Court material GS state: blend, alpha test, Z write, TFX, PRIM, wrap (in P17) → new `hst/src/gs.rs` gs.wgsl · `main.rs` gs_models · `mdl.rs` Packet.prim | t: gs.rs | j: 2026-10-07-p17-court-rendering
- [ ] **P17b** Fog (FGE) (in P17) → `gs.rs` gs.wgsl
- [ ] **P17c** VU1 lighting + HIGHLIGHT2 term (in P17) → `gs.rs` gs.wgsl · `main.rs` gs_models
- [ ] **P17d** Clouds: category-14 records (in P17) → `main.rs` load · `layout.rs`
- [ ] **P17e** Sky time of day + seasons `_sXXXX`/SSN1 (in P17) → `main.rs` load
- [ ] **P17f** Animated court textures MTA/UVA (in P17) → `gs.rs` · `mtl.rs`
- [ ] **P17g** Shadows: player/ball blobs, shadow models (in P17) → new
- [ ] **P17h** Mipmaps + LOD (TEX1) (in P17) → `main.rs` gs_models
- [ ] **P18** Upscaled + moddable textures → `main.rs` image · `character.rs` texture_image · `hst-data/src/tim2.rs` · `replacements/`
- [~] **P19** HUD as the original (user 2026-10-07): replace our player/score HUD with the game's own, from its assets: who is COM and who is a player, hidden during the rally; plus each player's marker above the head before the rally → `play.rs` hud/score_line · `hst-data` (AZUMA/INPANE) — BLOCKED (usage limit hit mid-task): serve panel ported (`play/panel.rs`: layout, slide-in/fade/ring timing, INPANE textures, faces, exe colours; matches slot-5 doubles screenshot) but the Sets/Games strip background and team-banner pill pieces sample wrong sub-rects; still to do: post-point score show (HUD mode 1), 'Score to win' line, head markers, in-match menus, singles/human checks, drop the old text score line. Notes: research/hud_rec.py, research/hud_shots.py
- [~] **P20** Audio (see N3) → `hst-data/src/xb.rs` | j: 2026-10-06-characters/1-MODELS-ANIM-AUDIO-PART.md
- [ ] **P21** Menus and modes → `main.rs`
- [ ] **P22** Widescreen, high fps, input polish → `main.rs` · `play.rs` read_input
- [ ] **P23** Graphics settings menu (after P21) → `main.rs`
- [ ] **P21a** Controller assignment screen (after P21): P1 picks which connected controller drives which player. The original shakes a controller's selector when that controller moves its right stick → `main.rs` · `play.rs` read_input/Pads
- [ ] **P24** Surprise pop-ups (user 2026-10-07): the "!" and sweat drop over an AI player caught off guard (AI only), and the sweat drop over a player who hits a serve that wasn't served to them; drawn in the same place and the same way as the original (sprite, position over the head, scale, timing, fade), checked against frame-stepped screenshots → `play.rs` balloons · `hst-sim` AI · `plan/P24.md`
- [ ] **P25** Ball-hits-player pop-ups (user 2026-10-07): when the ball hits a player it sometimes shows a sound-word pop-up ("thwip", "bonk", …); port when it shows and which one, drawn exactly as the original (sprite, place, scale, timing, fade), checked against frame-stepped screenshots → `play.rs` balloons · `ball.rs` player hit · `plan/P25.md`
- [ ] **P16a** Post-point cut-aways in play (low priority, user 2026-10-06) → `hst-sim/src/cutaway.rs` `flow.rs` `pose.rs` · `play.rs` camera/next_point · `tools/record_cutaway.py` | t: cutaway.rs | j: 2026-10-06-p16-cutaway

### Confirmations

- [ ] **P1** Shot parameters in play → `play.rs` strike/tables · `hst-sim/src/params.rs` `shot.rs` | t: shot_tables.rs, hst/tests/shot_params.rs | j: 2026-10-05-ball-physics/5-SHOT-PARAMS-READY.md
- [ ] **P2** Aim exactly (user 2026-10-07: a sweet-spot shot can't quite reach the exact corner as in the original; singles and doubles aim differently, their court lines differ) → `play.rs` aim_target/screen · `shot.rs`
- [ ] **P3** Timing grade effects → `swing.rs` · `play.rs` find_contact | t: swing.rs
- [~] **P4** All contact branches (dive, skeleton values, arm-IK step open) → `swing.rs` | t: swing.rs
- [ ] **P5** Shot selection by input → `play.rs` read_input/press/human
- [ ] **P6** Serve details, lets → `serve.rs` · `play.rs` serve_turn · `ball.rs` (bounce_turn) | t: serve.rs
- [x] **P15** Court collision mesh (mostly done by P0c3/4; check what's left) → `hst-sim/src/mesh.rs` `court.rs` `ball.rs` | t: live.rs
- [ ] N3c7 the lob sound seems to reverb weirdly after it should be done. might just be because we're missing other audio sounds so just verify it is correct

### Stretch

- [ ] **M1** Mod support: custom costumes for existing characters (texture swaps, model swaps), custom umpires, and fully custom characters, with a way to build them for the game logic from simple, easy-to-edit models (pick the best format) → `character.rs` · `hst-data/src/mdl.rs` `tim2.rs` · `replacements/` (P18)
- [ ] **M2** Online play: matches over the network (the sim is deterministic and tick-based, so input-only lockstep/rollback fits) → `hst-sim` · `play.rs` input
- [ ] **M3** In-browser version (wasm build) with online multiplayer (builds on M2) → `hst` app · `audio.rs`
- [ ] **M4** Music support (BGM is off by default; `--music` plays the court's): verify the BGM sequencer's note-on timing against the ring recording `context/recordings/bgm_s01.bin` (menu `bgmm_05` from save slot 1), live controller changes on sounding voices, the director's BGM restart and fade cues (messages 6/0xe/0xc), the menu BGM in a menu, jingle key 0 → `hst-data/src/snd.rs` · `audio.rs` `play.rs` | j: 2026-10-07-n3-audio/13-MUSIC-FINAL.md
- [ ] **M5** Our own upscaled textures: extract every texture unscaled from the disc (TIM2 in the archives, as loaded) and upscale them with the user's existing chaiNNer workflow (needs the human: where the chain lives and how to run it), then use them in place of `replacements/` (PCSX2 dumps) → `hst-data/src/tim2.rs` · `main.rs` image · `replacements/` (P18)
- [ ] **M6** Recreated font for text art: rebuild the game's lettering (set/game/match banners, pop-up words and the like) as a font and redraw those textures from it, crisp at any size, instead of upscales with artefacts. Match the original's letter shapes, colours, outlines and layout → `replacements/` · `play.rs` balloons/hud (P10, P12b, P19)
- [ ] **M7** Second camera mode (user 2026-10-07): port the original's other match camera (we play mode 0) as an option for single-player, or per player online when each has their own instance (M2). More camera modes could follow later → `hst-sim/src/camera.rs` · `play.rs` camera (P16)
- [ ] **P0b4c** Umpire-call timing (in P0b4) → `hst-sim/src/flow.rs` | t: score.rs | j: 2026-10-06-p0b-rules
- [ ] **P0b4d** Instant replay (in P0b4) → `flow.rs` · `play.rs` next_point | t: score.rs | j: 2026-10-06-p0b-rules

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

- [x] **P14c1** Trigger creature engine: type parameter table, path/waypoint motion, idle-animation timer, sound timer; idle animators 18, 34, 37, 44 → `hst-sim/src/npc.rs` · `exe.rs` | t: npc.rs
- [ ] **P14c3** Proximity-startled creatures (player/ball within reach: path + one-shot animation) → `hst-sim/src/npc.rs` | t: npc.rs
- [ ] **P14c4** Ball-hit obstacles (box overlap, speed threshold, message 0x14) → `hst-sim/src/npc.rs` | t: npc.rs
- [ ] **P14c5** Match-event creatures (game/set won, deciding set, per-point rolls) → `hst-sim/src/npc.rs` | t: npc.rs
- [ ] **P14d** Court 5 creatures → `hst-sim/src/npc.rs` | t: npc.rs
- [ ] **P14e** Walking spectators move (dodge, collision, ground, facing) → `hst-sim/src/npc.rs` | t: npc.rs

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

- M5 (stretch): where the chaiNNer upscaling chain is and how to run it.
- **Record a tiebreak (for P12)**: no save state or fixture reaches one. Play/record a bot match to 4-4 and through the tiebreak (incl. its 6-point end change and the set/match end) with `tools/record_p2m2.py <slot> context/fixtures/tiebreak.bin <frames>`, ideally from a save made at 4-3 or 3-4. Agents' RAM-poked attempts from slot 5 failed (PINE op errors mid-capture).
- **Doubles: who of two teammates gets the ball (for N5a)**: the port lets only one teammate hit (the first to lock a swing; the other's press whiffs, `play.rs theirs()`), from the recordings never having two teammates locked at once, but the original's code for this wasn't found. Record a doubles rally from slot 3 (P1 human + bot partner) where you and your partner both go for the same ball, pressing early and late, with `tools/record_p2m2.py 3 context/fixtures/partner_s03.bin <frames>`, so the rule (first lock, nearer player, or the human first) can be checked — j: 2026-10-07-n5a-lob-off-lob
- P13 umpire: record a long match (let, deuce again, set/tiebreak/match announcements) — j: 2026-10-07-p13-umpire/2-RECORDING-READY.md
