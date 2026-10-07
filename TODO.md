# TODO — Hot Shots Tennis remaster

## How to continue

When the user says **"continue"**: take the first unchecked prompt under **Prompts** and carry it out
completely as its own task, then tick it, commit, and stop to report. One prompt per iteration — do not merge
prompts or skip ahead. If a prompt turns out too big, split it into numbered sub-prompts in this file first.

Every prompt follows the same rules:

- **Match the original exactly — no approximations, no placeholders left behind.** Find the behaviour in the
  decompile (`context/decomp/`, `context/fn.sh <addr>`), port it, and prove it against the real game:
  record ground truth over PINE (`tools/pine.py`, `tools/trace_shots.py`, launch with `tools/pcsx2-hst.sh`)
  or from the user's save states (`~/Emulation/saves/ps2/states/SCUS-97610 (72326E67).NN.p2s`; slots 3, 4, 5
  belong to the user — only load them, save scratch states to 8/9). A prompt is done when a test compares the
  port to that recording and passes bit-exact (f32 via `hst_sim::ps2`) or frame-exact.
- **Arithmetic is PS2 arithmetic.** The EE FPU rounds toward zero and (as PCSX2 emulates it) masks the smaller
  addend to the exponent difference minus one guard bit before adding; `madd.s` = that add of the accumulator and
  a truncated product. Use `hst_sim::ps2` for anything that must match bit-for-bit.
- **Game data is read from the user's disc at runtime**, never committed (`hst_data::iso`, `xb`, `exe` — the last
  is the only place allowed to know retail *data* offsets, disc-ID checked).
- **Characters come from the user's disc.** (User decision, 2026-10-06: replaces the stand-in figures.) Players
  are the game's own models and motions, loaded at runtime from the disc (`crates/hst/src/character.rs`), never
  committed. The engine-side character format (joints with Biped names, skinned parts, clips by motion number)
  is the seam for custom characters.
- **Same input, same result.** Given the same controller input on the same frames from the same starting
  state, the port must produce the same hit spot, contact frame, timing grade, shot and ball path as the
  original. Prompt P0's input-replay harness is the acceptance test for every gameplay prompt.
- **Look at the original, too.** You can screenshot the real game any time (`tools/screenshot.sh`, PCSX2 paused
  or frame-advanced via PINE) and the port (`--shot out.png` or `tools/screenshot.sh out.png hst`), then compare
  them side by side at the same moment: animations, poses and their timing, effects, camera, HUD, pop-ups, menus,
  court rendering. Use it for anything visual and as a sanity check on gameplay work; numbers over PINE stay the
  proof where they exist. Keep the screenshot pairs in the prompt's journal folder.
- **Never stop while work remains.** See *When blocked* below — switch to other open prompts instead of ending.
- Record findings and evidence in `context/artifacts/<date>-<slug>/` (memory maps, decomp addresses, captures).
- Update **Done** and the **Play it now** controls when a prompt lands.

## When blocked

**Don't get trapped.** If anything stalls — a tool hangs, PCSX2 won't do what you need, the same fix fails
twice, ~20 minutes on one obstacle — stop, write what happened and what you tried in the prompt's journal,
mark the prompt `[~]` with a one-line reason, and move on to the next prompt. Never sit waiting on the human.

A prompt is **blocked** when it can't be finished this run: missing recording, needs the human (a decision,
root access, a physical controller), unresolved after real debugging, or the run is low on turns.

1. Commit whatever is solid; leave nothing half-applied in the tree (`git stash` experiments if needed).
2. Mark it `- [~] **Pn — …** BLOCKED: <one line why + what would unblock it>` and write the details and
   evidence to its journal entry.
3. Move to the next unchecked `- [ ]` prompt — preferably in a different section (Players, AI, Match,
   Presentation: camera, HUD, pop-ups, rendering, audio, menus), since unrelated work rarely shares the blocker.
4. Never end a run just because one prompt is stuck. Only stop when every prompt is `[x]` or `[~]`, and then
   list the blockers for the human.

Things that need the human go under **Needs the human** at the bottom, never into the code as guesses.

## Controlling the real game (for recordings and checks)

- `tools/pcsx2-hst.sh` launches PCSX2 with PINE on; `tools/pine.py` reads/writes RAM and loads/saves states.
- **Save states (the user's — load only, scratch saves go to 8/9):** 3 = start of a game, P1 human + 3 bots;
  4 = mid-rally right after the serve; 5 = full bot game. Slot 5 has no human input — use it for ball/AI
  captures only. Controller-input recordings (P0 and anything gameplay-from-input) start from slot 3 or 4 with
  P1 driven by `tools/vpad.py`.
- `tools/vpad.py serve` creates a virtual Xbox-360 pad (uinput, no root); PCSX2 binds it as `SDL-0` when no
  real controller is connected. `tools/vpad.py send "press cross 120" "stick l -1 0" "sleep 300" release`.
  Never press Select (PCSX2 hotkeys are Select + shoulder combos). Timing is wall-clock; frame-exact replays
  should use PCSX2 input recording (`.p2m2`) — P0 decides.
- `tools/screenshot.sh out.png [pattern]` captures a window (default PCSX2) without focusing it.
- Verify input effects numerically over PINE (e.g. ball/player state), not by eye.
- `tools/overnight.sh` (in tmux) runs `claude continue` (TUI: attach to watch or type; it never asks) back to back and owns the virtual pad for the night.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```

Doubles by default (`--singles` for 1v1). Player 1 = keyboard + controller 1, player 2 (player 1's partner; the opponent in singles) = controller 2
when connected; the other slots are CPU. WASD/left stick/d-pad move (and aim at contact, screen-relative) ·
J/A topspin · K/B slice · I/X flat · L/Y lob · U/RB drive · J/Space/A/Start serve ·
C/Select camera (original/free) · arrows/right stick turn the free camera.
`HST_AUTOPLAY=1` makes every slot CPU (unattended tests). `--stage 01..11` court, `--court 0..11` surface.
Gamepads whose device node is read-only (udev rules that strip write to stop rumble) work through the patched
`third_party/gilrs-core` (read-only fallback, no rumble).

## Done (ported and verified against the original)

- Disc/XB/TIM2/MDL/MTL readers; court layout placement; Bevy renderer, 60 Hz fixed sim + interpolation.
- Ball flight (drag, Magnus, gravity, curve/bend), ground bounces, rolling — 4524 recorded frames.
- Shot tables (TRAJ): lookup + launch speed/elevation/frames — 11 recorded strokes.
- Per-character shot parameter records (17 per class/kind, record = character + 3) built from GAME.BIN —
  bit-identical to the game's runtime table, using the PS2 FPU model.
- `hst_sim::ps2` — PCSX2's EE FPU model: chop rounding, add/sub alignment with one guard bit, **div and sqrt
  round to nearest** (the emulator's divider mode), DAZ. Proven on 2912 table values and the ball integrator.
- Ball flight **and bounces** on PS2 arithmetic in the original's instruction order: all 4524 frames of 39
  recorded shots (flight, curve/bend, every bounce, rolling) bit-exact in position and velocity. Pieces:
  `hst_sim::vu0` (VU0 chop model), `contact` (plane sweep, contact point — the original lerps to the raw hit
  fraction; its 0.005·r back-off only gates the ≤ 0 test), `quat` (matrix↔quat, VU0 slerp microprogram),
  `libm::sinf` (the game's fdlibm sinf, incl. its pio2_2 = 0x373543ff).
- Net contact (flat net from the game's predictor), material-based bounce response.
- Collision world placement: props from plant records (VU0 sin/cos, rotations, inverse) and the 20 m prop grid —
  117 props and every grid cell bit-exact on court 10.
- Live ball against the world mesh (court model, walls, net, props via the 20 m grid), ground material from attribute
  maps, material table response — 12498 recorded frames of a bot match bit-exact, all contacts included.
- First-bounce turn of serves (`+0x1b0`, game's atan2f) — round1's turned serves bit-exact; `--stage` play on
  the world mesh.
- Live-ball point verdicts (`hst_sim::judge::Rally`) — every decision of a recorded bot match frame-exact, rally
  block equal every frame (faults and net points included).
- Post-point scoreboard timeline (`hst_sim::flow`): pause, wait, score shows, change ends — every call-free
  point-over phase of a recorded bot match tick-exact.
- Stroke contact search + timing grades (SWEET SPOT / QUICK / SLOW) — ground-stroke branch.

## Prompts

### Next (user priorities, 2026-10-06 — do these first, in order)

- [ ] **N1 — Accurate, functional animations & also original movement speed.** Replace the motion picking in `play.rs` (`motions`) with the
  game's own: the motion setter `0x350280` (motion number, speed, loop/hold flags) and its callers per player
  state (ready `ad00`/`ad01`, turns `tb_f`/`tb_b`, runs/dash by the stick and speed, receive, strokes and their
  follow-through, serve stance/walk/toss/swing, whiffs, reactions `re_*` after points), motion blending
  (`0x140360`/`0x1404c0`), the contact pose (frame 8) and the arm IK / body step at contact (`0x3561e0`, +0x3fd0/
  +0x3fd8), faces (`.MOR`/`.UVA`). Verify per frame against a PINE recording of each player's motion number and
  time (player +0x54 motion object: number +0x20, time +0x34/+0x38) in the slot-5 match and a slot-3 human game.
  Split (journal `context/artifacts/2026-10-06-n1-animations/`):
  - [x] **N1a — Locomotion.** `hst_sim::player`: run speed (TParam SPE/10, Agili acceleration +30 %, stamina
    slow-down), run counter, stamina drain, stand/run motion choice (ready, turned stances to the ball, run
    f/b/l/r with hysteresis, dash, tired +8), game `acosf`. `tests/player.rs`: run velocity bit-exact (9657 frames),
    motion exact on all 70575 stand/run frames of `match_s05.bin`. Wired into `play.rs` (`locomote`; per-character
    stats; mover bounds). Then: the mover (`34a960`: 1 m doubles partner push with on-court fallback, bounds,
    half-court mode) bit-exact on 69701 ticking stand/run frames; stroke stamina costs (TParam dive/backhand/smash,
    `35ae50`) on every recorded drop; weather 2/3 agility ×1.5 (`Stats::new`; the app passes clear weather until
    the weather table is ported). The partner push never fires in `match_s05.bin` — unit-checked only; verify with a
    slot-3 recording where P1 runs into its partner. Journal `1-LOCOMOTION-FINAL.md`.
  - [ ] **N1b — Facing turn.** The rest of `349410`: turning +0x3d60 toward +0x3dc0 (turn flag +0x3dd1/+0x3dd0, the
    motion's root matrices at player + motion·0x40 + 0x6b0); verify facing every frame of `match_s05.bin`.
  - [ ] **N1c — Mode state machine.** When each mode/motion applies: stroke mode 2 (`3ec4` countdown, recovery
    +0x3e50, re-press), mode 3, serve motions, receive, reactions `re_*`, `3553d0`; verify the motion number on
    every frame of `match_s05.bin` (not only stand/run frames).
  - [ ] **N1d — Motion playback.** Motion time/speed (+0x34/+0x38), blending (`0x140360`/`0x1404c0`), contact IK
    and body step (`0x3561e0`), faces (`.MOR`/`.UVA`); record motion time over PINE to verify.
- [ ] **N2 — The original ball when serving.** The ball sits in the server's hand and follows the serve motion's
  ball track (`sh_pcNN_serve_ad00_ball`, `*_serve_t_dummy`) until the toss leaves the hand bone; drawn with the
  game's ball model (`CMN/GAME.XB` `ball1.mdl`) and shadow instead of the toon sphere. Verify the ball position
  per frame through a serve against a recording.
- [ ] **N3 — Audio working.** Decompile the IOP sound driver (`IRX`) to resolve note → tone (see P20 journal), then
  play the game's sounds at its trigger points: racket hits (court bank program 6 by grade/type, `0x340860`),
  bounces, serve, character voices (per-player banks), umpire calls, crowd, music. Verify trigger frames against
  recordings.
- [ ] **N4 — Landing markers as the original.** Replace our red dot with the game's own bounce marker (texture,
  size, colour, alpha/pulse, when it appears and disappears; `chakudan` in `AZUMA/C_EFF` EFFCT.XB0 is a likely
  source), and add the yellow outline the original draws where a lob will land. Verify against frame-stepped
  screenshots of the original (`tools/screenshot.sh`): same frames, position, size and colour.
- [ ] **N5 — Lob smash.** Let a player smash a lob (overhead on a high ball), as the original: the smash
  branch of the contact search (`hst_sim::swing::search`, P4) for lobs, the smash shot table, its motion and
  the movement to get under the ball. Verify a recorded lob smash's contact frame, branch and ball launch.
- [x] **Player 1 is Carol** (character 6, left-handed) by default; `--chars` still overrides. Note: the contact
  search and serve still use character 0's reach/timing data for everyone (P7/P3).

### Harness

- [x] **P0 — Input replay harness.** Fixture reader `hst_sim::replay` (pad, globals, match, ball, player
  position = model matrix translation +0x3d70/+0x3d78), CLI `cargo run -p hst-sim --bin replay -- <fixture>
  [from] [to]` (CSV per frame), recorder `tools/record_p2m2.py`. round1 (`context/fixtures/round1.bin`, doubles,
  20791 frames, vsync 10018–30808, frame-exact, complete) replays score (29 points), verdicts (30) and line
  calls (158, bit-exact); `tests/replay.rs` checks the fixture itself. Input-driven player diffs land with P7
  (the port has no ported movement yet; P7 builds them on `Frame::{pad, player_pos}`). Next recording: add the
  live ball `*(gm+0x88)` and the rally block 0x3165f0. Journal: `context/artifacts/2026-10-06-p0-input-replay/`.

- [x] **P0a — Bit-exact bounce.** Port the contact path with the original instruction order so
  `crates/hst-sim/tests/flights.rs` can require bit-exact position *and* velocity on every frame (today: airborne
  frames exact, first miss at the first bounce). Pieces: plane sweep `0x328ed0` (contact skin 1.005×r, eps 0.005,
  0.98 factor, VU0 dot products), contact point `0x12fc30`, VU0 helpers (normalize `0x125b10` with Q sqrt/div,
  cross `0x125ac8`, mat×vec `0x125a50`/`0x125a80`, inverse `0x32d190`), quaternion slerp (`0x12d620`, `0x12d860`,
  `0x12d340`) and the game's own sin/cos/acos (`0x115fb8`, `0x115d18`, …). Needs a `vu0` arithmetic model
  (PCSX2 VU path: chop, no add alignment — verify like `ps2`). Asm: `context/ghidra_scripts/DumpAsm.java` →
  `context/notes/asm_*.txt`.

- [x] **P0c — Live net hit, bit-exact.** Record a natural net contact (bot match, slot 5) per frame with
  `tools/trace_live.py`-style capture and replay it through `Flight` bit-exact. Needs the live collision path:
  world mesh query instead of the flat net (court object + grid objects, per-model triangle sweep) and the
  after-first-bounce redirect `+0x1b0`. Findings so far: `context/artifacts/2026-10-06-net-hit/FINDINGS.md`.
  Split (journal `context/artifacts/2026-10-06-net-hit/`):
  - [x] **P0c1 — Live-ball recording + per-frame replay.** `tools/record_live.py` (slot 5, every frame at
    0.25 speed: live ball `*(gm+0x88)`, predictor `*(gm+0x98)`, rally block) → `context/live/net_s05.bin`;
    `crates/hst-sim/tests/live.rs` steps each recorded frame through `Flight`: 12309 airborne frames bit-exact;
    189 contacts listed (court 144, fence 19, outside ground 12, net 5, net cord 3, 6 uncounted pushes) — court
    bounce velocity already exact, position off ~1e-4·r. Journal `1-LIVE-RECORDING-PART.md`.
  - [x] **P0c2 — Collision world data.** The live ball queries the court object and the world grid's props (net =
    `znet_s1000.mdl`, umpire chair, seats, hut, gate, trees); each is its `.mdl` loaded in place. `mdl::Model::collision`
    / `colliding(&mtl)` (static batches of materials with an attribute map; order node → material → batch → strip slot,
    restarts skipped), `node_inverse`; `mtl::Material::{two_sided, attributes}`, `Mtl::attributes` (embedded textures +
    16-byte material table). `crates/hst-data/tests/collision.rs`: slot 5 RAM vs disc, 15 models / 6368 triangles,
    node matrices, flags and maps bit-exact. Journal `2-COLLISION-DATA-FINAL.md`.
  - [x] **P0c2b — Collision world placement.** `hst_sim::world`: plant records (categories 15, 17–20, model, scale ≠ 0)
    → `place` (yaw, code-byte tilt/turn, net post at the origin) → `instance` (scaled node matrices, world→model,
    bounding sphere) in the game's list order (`list_order`), and the 20 m `grid` of colliding props.
    `crates/hst-sim/tests/world.rs`: 117 props / 66 colliding, matrices, spheres, grid box and all 704 cell lists
    bit-exact vs slot 5. Also fixed `vu0::add` (exact round-toward-zero for far smaller addends). Journal
    `3-PLACEMENT-FINAL.md`.
  - [x] **P0c3 — Triangle sweep bit-exact.** `hst_sim::mesh`: swept sphere vs triangle (face, edges, corners), the
    attribute-map ground material (texel at the hit's UV; `mdl::Model::wrap` = GS CLAMP modes), the per-object sweep
    (world→model, VU0 strict box test, two-sided passes, box shrink after a hit) and the world query (court, then grid
    props with the sphere pre-test). `Flight::step_world` + `ball::Material` = rows of the game's material table
    (`exe::Game::surfaces`); ghost material 32 (passes through, uncounted). `tests/live.rs` steps **all 12498 frames
    bit-exact** against court 10 built from the disc, contacts included (court 144, outside 12, walls 19, net 5,
    cord 3). Fix on the way: off-court spin loss also resets the kick's "spin before". Journal `4-TRIANGLE-SWEEP-FINAL.md`.
  - [x] **P0c4 — Net through the mesh in live play.** First-bounce turn `+0x1b0` (`0x379bd0`; per-character serve
    table, game's atan2f/atanf in `libm`) on `Shot::bounce_turn`, gated as the original (rally on, first sub-step,
    first bounce): round1's two turned serves now bit-exact, `line_calls.rs` exemption removed. `hst_sim::court`
    builds any court's collision world from the disc (multi-node props: local·parent); `--stage N` play steps the
    live ball through `step_world` (flat net only without a stage). `0x379ae0` (random bounce option) left for P21.
    Journal `5-LIVE-PLAY-FINAL.md`.

### Match basics (do these early)

- [ ] **P0b — Tennis rules and serve flow.** Proper match flow before polishing anything else, exactly as the
  original: server serves from behind the baseline alternating deuce/ad sides, the serve must land in the
  diagonal service box, faults and double faults, second serve, receiver positions, server rotation each game,
  point → game (deuce/advantage) → set scoring, side changes, and the original's serve timing: toss height and
  duration, the contact window and how the press timing during the toss affects the serve (port `37af20` and the
  serve branch of the hit routine `3467b0`), plus the delays between points. Verify against recorded serves
  and full points from the save states with P0's replay harness. (P6/P12 cover the remaining details.)
  Part 1 done: score, rotation, ends (`hst_sim::score`, wired into `play.rs`); its round1 replay test only
  covers the points captured so far. Part 2 done: line calls, point-over check, hit legality, umpire verdict
  (`hst_sim::judge`, in `Flight` and `play.rs`): 158 recorded calls bit-exact, 30 verdicts match. Faults/lets/
  net points are unverified until a capture includes the live ball `*(gm+0x88)` (the round1 capture's ball is
  the path predictor) — see journal `2-JUDGE-PART.md`. Rest split (journal `context/artifacts/2026-10-06-p0b-rules/`):
  - [x] **P0b3 — Live-ball verdicts.** `tools/record_p2m2.py 5 …` (fixture + live ball + rally block 0x3165f0) →
    `context/fixtures/match_s05.bin` (whole slot-5 match, 26400 frames); `tests/score.rs::match_s05_rally_block`:
    hit check and point-over check on the game's frames (the check sees the ball and body hit as of the previous
    frame), rally block equal every frame, 36 decisions (26 points, 8 faults, 2 out after the net) frame-exact
    with the recorded winner. Lets, double faults, illegal hits, rally outs not in this match — still unverified.
    Journal `3-LIVE-VERDICTS-FINAL.md`.
  - [ ] **P0b4 — Post-point flow.** Phase/sub-phase timeline (gm+0x55/0x56) from the decision to the next serve.
    Split (journal `4-POST-POINT-FINAL.md`):
    - [x] **P0b4a — Scoreboard timeline.** `hst_sim::flow::PostPoint`: pause 30, react, wait (point 20 / game 90,
      GAME.BIN), score show per event (point/tiebreak/game/set stages, deuce variants, hold 1 s / 2 s, fade out),
      faults cleared at the pause's end, points/games cleared when the game/set show starts, change ends → phase
      1 (`CHANGE_ENDS` = 80, camera cut) or serve. `tests/score.rs::match_s05_post_point`: all 25 call-free
      point-over phases + 2 change-ends phases tick-exact (gm+0x58; vsync stalls while the game loads don't count).
      Set and tiebreak shows ported from the code, not yet in a recording.
    - [x] **P0b4b — Placement for the next point.** `hst_sim::flow::serve_placement`: server on the baseline at
      its last toss's distance off centre (3.0 on the match's first point), receiver 3.0 off centre on the baseline
      (11.0 deep for a second serve), partners 2.7425 off centre at 3.2 (server's) / 5.2 (receiver's) or 7.9 in the
      back formation; facing by player parity, ends and the start flip. The singles ad-side receiver index names an
      absent player, so that receiver takes the partner spot (game quirk, ported). `tests/score.rs::
      match_s05_serve_placement`: all 4 players, position + facing bit-exact on all 37 set-ups (35 serve entries,
      2 change-ends entries; a serve entry after change ends keeps that placement). Wired into `play.rs`
      (`reset_positions`, stance remembered at the toss). Ends are not yet swapped on court in the app: `Side` is
      both player and end there → P0c. Formation choice (first point, doubles, RNG) and the message-0xe pre-serve
      motion (RNG 50%) → P0c / P8. Journal `5-PLACEMENT-FINAL.md`.
    - [ ] **P0b4c — Umpire-call timing.** Faults/outs/lets/double faults: scoreboard call show (`388190`, 0x427 →
      chained score show with a 1-tick pause for out / out-after-net / double fault) ends when the call sprite
      animation is done and the umpire voice has stopped (fallback countdown table 0x410f0c by call × language);
      recorded faults take 79 or 80 ticks (voice jitter). Also the match-over wait (`326270`: voice line, then
      phase 5). Needs the sprite anim length from disc and the voice clip lengths (P13/P20).
    - [ ] **P0b4d — Instant replay.** The decision in `325cd0` (gm+0x32e; ball speed, shot count, options
      0x2ef2b6) and the replay run (saved block 0x4230d0/0x316640 restored, fast-forward/slow-mo ticks).
  - [x] **P0b5 — Serve.** Done: `hst_sim::serve` (walk, three tosses, toss launch, serve contact search, box aim
    with strong-toss mistiming errors), `tests/serve.rs` 36 recorded serves exact. Shot parameters → P6. Original: Toss height/duration (`37af20`), the contact window and how the press timing during the
    toss changes the serve (serve branch of `3467b0`). Verify with vpad-driven serves from slot 3/4 (pad + ball
    - player state over PINE) through the replay harness.
- [~] **P0c — Up to 4 players.** Playable doubles: 4 slots, placement, rules, P1/P2 on controllers 1/2, CPU
  for the rest, ends swap on court. Open: the original's partner positioning / who-takes-the-ball, formations,
  shared-camera behaviour, hot-plug beyond two pads. Original text: Four player slots, each human (keyboard or any connected controller) or AI;
  singles (1v1) and doubles (2v2) in any human/AI mix, as the original allows. Controller assignment and
  hot-plug per slot, doubles court width (5.485 m) and doubles rules, partner positioning and who-takes-the-ball
  logic from the original (port it; doubles AI data is in AIParam.csv, e.g. the 雁行/攻撃/守備 formation column),
  serve/receive rotation for doubles, and the original's shared match camera behaviour with several humans.
  Verify with slot 3 (P1 + 3 bots doubles) recordings through the replay harness.

### Shots

- [ ] **P1 — Shot parameters in play.** Replace `KIND_SPIN` and every per-shot constant in `play.rs` with the
  character's record (`hst_sim::params`, record = character + 3) and the character's own TRAJ tables
  (class/kind/record → file, as mapped from the `gm+0x9c` table owner). Port `37e420` (normal↔charged blend),
  `37e950` (launch frame, spin, curve/bend from `37e240`, frame count adjustment) and `379080`/`375da0` (launch,
  including the two MT19937 draws). Verify launches against recorded shots (`context/shots_s05b`).
- [ ] **P2 — Aim exactly as the original.** Port `37b110` + `37f740`: base target from the hit routine,
  aim correction (court lines, net clearance), and the analog stick offset (`stick × 1.5 m`, rotated into the
  shot frame). Verify recorded shot bearings (the leftover ~0.2–1° offsets must disappear).
- [ ] **P3 — Timing grade effects.** What grade/offset change in the shot (power code → table variant
  `_dw/_up`, `37b110` param_3 mapping, `3467b0` branches on `+0x3ee8` / `+0x3fa0`, reactions via `3553d0`).
  Remove the approximate wind-up reach growth in `find_contact` by porting the exact reach term. Verify recorded shots at each grade (QUICK/SWEET SPOT/SLOW, early and late
  offsets): launch and grade bit-exact through P0's harness.
- [~] **P4 — All contact branches.** Ported (`hst_sim::swing::search`): smash, volley (high/low/body) and ground
  stroke (by shot type/body) branches, forehand/backhand by hand side, timing grades; `tests/swing.rs`: all 131
  stroke decisions of `match_s05.bin` (55+22+9 ground, 11+23+2 volley, 9 smash) frame, branch, side, body flag and
  contact ball exact (ball within 1e-5). Open: the dive/reach branch (+0x3e58), the swing animation's
  skeleton-derived values (`ahead`, body widths, anchors +0x2f60/+0x3000/+0x3028 — measured for character 0) and
  the arm-IK sideways step (stand-in 0.1 m; P8). Original text: Port every branch of the contact search in `0x34d8a0`: forehand/backhand
  ground strokes (body-shot variants `0x16/0x18/0x1a`), volleys, smash (1.85–2.65 m, reach 1.1), the
  reaching/diving shot (offset −10), half-volleys, and their per-character timing tables (`+0x1510`, `+0x1644`, …). Verify a recorded contact for every branch (volley, smash, dive, half-volley, body
  shots): contact frame, hit spot and branch frame-exact through P0's harness.
- [ ] **P5 — Shot selection by input.** How buttons, hold/charge time and stick map to class/kind/power and the
  charged blend; double-tap / combo inputs; special shots (`0x408e60` list). Verify with recorded inputs
  (capture pad state over PINE alongside shots).
- [ ] **P6 — Serve details.** (Basics in P0b.) **Turned serves**: `Flight` applies `Shot::bounce_turn` (P0c4); the app
  must fill it from the shot-effect table (`0x37e240`: per-character 0x404110 row, class 0 kind 0, special condition
  `0x37f660`/0x410870, doubles exception) along with curve/bend. Real toss (`37af20`), serve tables `serv0..3`, serve timing/power meter if any, faults,
  service box rules, second serve, serve positions per side/court. **Lets:** a serve that clips the net cord
  and lands in the box is a let and is replayed; one that clips the cord and lands out is a fault — the cord
  contact must come from the exact net collision (P15), and the let/fault call and replay flow must match the
  original (verify with recorded let serves).

### Players

- **Handedness:** characters can be left-handed; **Carol and Will are lefty by default**. Read handedness from
  the game's character data (don't hard-code names) and mirror everything that depends on it: forehand/backhand
  side, contact search reach/offsets, swing and serve animations, toss hand, racket attachment.
- [ ] **P7 — Exact character movement stats.** Every character moves exactly as in the original, per
  character: walk/run speed, dash speed (if any; the original has no sprint button), acceleration and deceleration curves, turning rate, direction
  changes, split-step/ready hop, recovery after a shot, reach (forehand/backhand/volley/smash, e.g. the
  per-player `+0x13a0..0x13d0` fields), contact height windows, serve position and movement before/after serving,
  dive distance, and how stats like *Run CON* / *Body ADJ* / *Back ADJ* / *Rizing ADJ* in TParam.csv (and the
  player object derived from them) turn into those numbers. Read every value from the disc at runtime (TParam.csv,
  GAME.BIN via `hst_data::exe`), never hard-coded. Auto-positioning toward a locked contact must follow the same
  rules. Verify each character (at least Kaito pc03 and Carol pc06 first, then all 14) by recording player
  position/velocity every frame over PINE with scripted `tools/vpad.py` input and replaying the same input
  through P0's harness — positions must match frame-exact. Document the derived per-character stat table in the
  journal (not in git).
- [~] **P8 — Player animation timing.** Models + motions from the disc, motions chosen by the game's numbers
  (serve, strokes timed to the contact pose, runs). Open: the game's exact motion state machine and blending,
  MOR/UVA (faces), arm IK at contact, reactions. Original text: Port the player state machine and the animation timing from the game's
  ANI/ANI2/MOR data and code: idle, ready, run directions (`run_f/b/l/r`, `dush_f`), shot animations
  (`sh_*`), contact frames, recovery, celebrations/reactions. Drive the stand-in figure's poses from those exact
  states and frame timings (stand-in visuals, original timing). Verify the state id and animation frame every
  frame against recorded player state over PINE (run, every shot, recovery, reactions) — frame-exact.
- [ ] **P8a — Missed swings (whiffs).** Pressing a shot button when the contact search finds no ball (out of
  reach, too early/late, ball not in play) must swing at nothing exactly as the original: the miss branch of the
  hit routine / contact search (`3467b0`, `0x34d8a0`), which swing animation it plays (forehand/backhand/volley/
  smash variant, by ball side and height), its frame timing, and the movement lockout — the frames the player
  can't move or swing again after whiffing, plus any slowed recovery. Also whether a ball arriving during the
  whiff can still be hit. Verify with recorded whiffs (pad + player position/state over PINE): lockout start/end
  and positions frame-exact through P0's harness.
- [ ] **P9 — Hit effects.** Hit flashes, ball trails, impact/bounce effects and their timing from `AZUMA/C_EFF`
  (EFFCT.XB0: `impact_*`, `ballbound_*`, `smash_*`, `chakudan`), including blend modes and UV animation (.UVA, .MTA). Verify spawn frame, position and lifetime of each effect
  against recordings and side-by-side screenshots (`tools/screenshot.sh`) at the same frames.
- [~] **P10 — Timing pop-ups as the original.** Balloons (sweet/note/bunny/turtle) from the disc with the game's
  choice rule and fade timing; exact screen size/position vs frame-stepped screenshots still open. Original: Use the game's own pop-up textures and animation (find the
  sprite/texture for SWEET SPOT / QUICK / SLOW-style feedback in `AZUMA/INPANE`, `CMN`, `MENU` archives and the
  code that animates it: scale/fade curves, position, duration) instead of our text pop-up. Verify against frame-stepped screenshots of the original: same frames,
  position, scale and alpha.

### AI

- [ ] **P11 — Opponent AI.** Port target choice, shot type choice, positioning, reaction delay and timing error
  from the AI code and AIParam.csv per character; it reads the stored path (path recorder: 15 steps/frame,
  court plane only). Verify AI decisions against slot 5 (all-bot) recordings.

### Match

- [ ] **P12 — Rules and scoring details.** (Basics in P0b, doubles in P0c.) Games, sets, deuce/advantage, tiebreak, side changes, doubles rules, match
  flow states (point start/end delays, replays if any) exactly as the original. Verify full recorded games/sets
  (incl. a tiebreak and a side change) through P0's harness: score and state transitions frame-exact.
- [ ] **P12a — Post-point sequence and score update.** Everything between the point ending and the next
  serve, exactly as the original: who won and why (winner, out, net, double fault, missed return), the players'
  post-point animations and reactions (winner celebration / loser frustration variants, by character and point
  type, from ANI/ANI2 + the state code; stand-in figures, original timing), walking back to positions, the
  point-end and replay cameras (with P16), the moment and animation of the score update (HUD score change,
  game/set announcements, umpire call with P13), crowd reaction, and the delays until control returns. Verify
  recorded point endings of every type through P0's harness: state transitions, animation choice, score-change
  frame and next-serve frame all frame-exact, plus frame-stepped screenshots for the HUD update.
- [ ] **P12b — Match pop-ups as the original.** (User request 2026-10-06: score count, fault, net, service ace
  and the rest.) Scope still open with the user: exact-only, or the disc textures + exact timing now with the
  call models as flat fading sprites until P12b2 (each gap a sub-prompt). Not started in code; another session
  had uncommitted character-model work (`character.rs`, `ani.rs`, `mdl.rs`) in the tree — put this in its own
  `crates/hst/src/popups.rs` with a small hook in `play.rs`.
  **Textures** (all in `AZUMA/INPANE/INPANE.XB0` unless noted; names listed in the program at the inpane loader
  `3a48d0`, archive `azuma/inpane/inpane.xb`):
  - Calls, models `azuma/inpane/mdl/<name>.MDL` + .ANI/.MTA/.MOR/.MTI (table of {name, 6 flag bytes} at
    0x4159a0, 12 bytes each, 6 entries, loaded by `38e870`): `i_let_00` (Let, blue, no ANI), `i_out_00` (Out, no
    ANI, has MOR), `i_net_00` (Net, ANI+MOR), `i_fault_00` (Fault), `i_doublefault_00` (Double Fault, 512×64),
    `i_coatchange_00` (Change / Sides, 256×128, ANI+MOR).
  - 2D sheets `azuma/inpane/mtl/` (table of {name, arg} at 0x4159f0, 8 bytes each, 0x30 entries; entry 0x10 from
    `saitou/hsmode/mtl`, 0x11 from `menu/2d`): `inpane_kihontokuten00` (yellow 0/15/30/40/Deuce/Advantage, 128×64
    cells), `…01` (white, same layout — shadow/second pass), `inpane_duce00`/`01` (Deuce! yellow/red + digit
    strips 0–9 ×), `inpane_tiebreak00` (red 0–7 / Deuce / Advantage), `…01`, `…02` (Tie break), `inpane_set`
    (Set Point), `inpane_match` (Match Point), `inpane_finish00` (Untouchable Service Ace), `…01` (Untouchable
    Return Ace), `…02` (Untouchable Smash Ace), `…03` (On the Line), `inpane_finish` (256², unchecked),
    `inpane_Counter` (Counter), `inpane_p` (1P 2P 3P 4P COM), `inpane_sen1` (wipe bar), `inpane_mini0/1`
    (Games: 0123456), `inpane_team`/`team1` (A Team / B Team), `result_game`/`result_set`/`result_gameset0x`.
  **Scoreboard object** `*0x42d6c0` (built by `382520` from `3a4fb0`, handler `383350`, update `382810`):
  sprite pointers at +0x74…+0x110; sprite struct +0x10 x, +0x14 y, +0x18 w, +0x1c h, +0x20 u, +0x24 v,
  +0x28 uw, +0x2c vh, +0x50..+0x5c RGBA (0x80 = 1.0). Screen space is the PS2's (640×448 assumed — check).
  Show start `384280(sb, kind)`: +0x148 = kind (1 point `384fa0`, 2 `3874b0`, 3 game `387910`, 4 set `387d10`,
  5/7 call, 6 tiebreak `3891d0`); kind ≠ 0 is skipped while an instant replay is pending (gm+0x32e). Call (5):
  +0x58 = call model (+0x5c + 4·call 0x427), all six reset to frame 0 at scale 1.0 (`38eeb0`), countdown
  0x42d6e8 from table 0x410f0c (see P0b4c). Call over when `38ee70` (anim time ≥ end) and voice idle.
  **Point show** update `384c00`, draw `384490`: fade-in/out alpha = 128·t/5 (t 5→0) on +0x74…+0x110; roll
  steps +0x164 0..4 (deuce: 3..1), settle fade of +0x94/+0x9c over 15 (alpha 128·t/15); +0x2f4/+0x2f8 blink 20
  ticks on/off. Draw (singles/doubles ≤ 2 teams): score digit sprite +0x90 at x 336, y 104 + 180·row (row =
  team, flipped when +0x190 ends swapped), 128×64 from UV cell (col table 0x410fe0, row table 0x411000, index =
  team's points 0x422f90[+0x35], the scoring team's −1 while rolling, 4 in tiebreak-deuce); the scoring team's
  new value wiped in from x 336 + off, width 128 − off, off = (0x411020/24) − 25.6·step, via `3a6e70`; +0x94 is
  the settled overlay. Background bars `3a6f40` (x 216/248 or 232/264/216 for doubles, y around 0x110−176·swap
  and 0x60+176·swap, +0x14 for < 3 players). Then `38d500(sb, i)` for i = 0, 1, 0x23, 3, 4, 9, 10 (names, faces,
  serve marker? — unread). Doubles/team mode (0x316620) → `387da0`.
  **Still to read**: game/set shows (`3876c0`, `387ac0`, `387260`), tiebreak (`388ec0`), call draw (+0x58 model:
  position/projection), `38d500` elements, Deuce!/Set Point/Match Point triggers, the finish banners' trigger
  (ace = untouched serve/return/smash; On the Line = line-call margin?) and draw, Counter, Change Sides with the
  phase-1 cut. Model animation players: node ANI `140360`/`1404c0`, material `14f3f0`/`14ef50`, MOR `156010`/
  `155ba0`, plus `155020`, `150260`, `151200` (UVA?) — none ported (`hst_data::ani` reads only ANI2).
  Verify: frame-stepped screenshots of the original vs `--shot` at the same ticks (position, size, alpha), and
  the show timings already covered by `tests/score.rs::match_s05_post_point`.
- [ ] **P13 — Umpire (Lily).** Port the umpire's behaviour: calls (score, fault, out, let, net) and their timing,
  chair placement per court, idle/turn reactions, voice-line triggers. Stand-in figure for visuals. Verify call type and frame for every recorded call (score,
  fault, out, let, net) against the original.
- [ ] **P14 — Background NPCs.** Spectators/ball kids/creatures from the layout's creature/gallery records
  (categories 21/23) and their animation timing (`azuma/gallery/ani/npcNN_*`, `trgCreMdl`) — positions,
  paths and triggers exact; stand-in figures for visuals. Verify NPC positions and trigger frames against PINE
  recordings on at least three courts.
- [ ] **P15 — Court collision mesh.** Live-ball collision against the court mesh (`FUN_0032f690` on `gm+0x84`):
  net cord/posts, walls, fences, other materials; replaces the flat net. Net-cord hits must behave exactly:
  balls that clip the cord and dribble over (rally net cords and serve lets), balls stopped by the net, post
  hits. Verify with live-ball traces (the stored path ignores the net, so record the live ball `*(gm+0x88)`; `tools/record_live.py`).
- [ ] **P15a — Per-court bounce profiles.** Every court bounces the ball exactly as its original surface does
  (hard, clay, grass, carpet, … — whatever the game defines). Today `hst_sim::ball::COURTS` hard-codes only
  `spin_relax` and `spin_kick` per court and shares friction 0.3, spin→speed 0.2 and restitution 0.7 across all
  12. Find the game's full per-court surface table (every field the bounce reads, plus any per-court values
  elsewhere: rolling threshold/relax, first-bounce scales, material rows for that court's walls/fences) and read
  it from GAME.BIN at runtime via `hst_data::exe` instead of the consts. Pin the disc court folder (`--stage
  01..11`) ↔ physics court index (0..11) mapping from the game's own lookup (today unverified, see *Known gaps*)
  so `--stage` picks the right surface automatically; keep `--court` as an override. Verify one recorded rally
  with several bounces (incl. topspin, slice and lob kicks, and a roll-out) on every court through P0's replay
  harness — bounce frames bit-exact on each surface.

### Presentation

- [~] **P16 — Camera.** Match camera mode 0 ported (`hst_sim::camera`, update `369ce0`): orbit 41 m / 17° /
  framing offset, smoothed dolly-back for the near team (`36a4a0`), bottom fit (`36acc0`), sideways slide + widen
  (`36a840`), 10 % easing, ball top fit, cut on each new point, turned round behind a human on the +z half.
  `tests/camera.rs`: all 4883 serve/rally frames of `camera_s05.bin` within 4 mm (f32, not the PS2 sequence).
  Shown fov: game horizontal half-angle of 4:3, vertical ×0.75, never less than 4:3 in any window. Open: other
  modes (table 0x3fcd4c rows 1+), point-over/replay/cut-away cameras (+0x60/+0x64), two-human behaviour. Original text: Port the original in-match cameras exactly: broadcast/follow angles, FOV, smoothing,
  serve/replay/point-end cameras (`camed/cam_cNN_*.dat`, `.CAM`), per court. Keep our free camera as an extra. Verify camera position/target/FOV every frame against the game's
  camera state recorded over PINE (rally, serve, point end, replay) — bit-exact where the math is ported.
- [ ] **P17 — Court rendering fidelity.** Material blend modes and flags (MTL header), vertex colour/lighting,
  cloud placement (category 14 records), sky time-of-day variants, seasons (`_sXXXX`, `SSN1`), animated
  textures (MTA/UVA), shadows (incl. the per-frame shadow blobs), fog. Verify with side-by-side screenshots of the same court and camera
  (original via PCSX2 at native resolution vs. port) and document any remaining difference.
- [ ] **P18 — Upscaled and moddable textures.** The remake must use upscaled textures and let the user edit
  them. (1) Map the user's `replacements/` pack (PCSX2 hash-named PNGs) onto disc textures (compute PCSX2's
  texture hash from TIM2/MTI data + CLUT) and load them in place of the originals, any size, with alpha.
  (2) Add a mod folder of readable names (archive/texture path, e.g. `mods/textures/COURT/05/<name>.png`) that
  overrides both the pack and the disc, plus a `--dump-textures` command that writes every disc texture there
  (with its matching pack PNG if one exists) so the user can edit and drop files back. Lookup order: mod
  folder → PCSX2 pack → disc. Reload on file change while running. Same rules as the disc: user files are read
  at runtime, never committed (add `mods/` to `.gitignore`). Verify the computed hash matches PCSX2's dump
  name for every texture the pack covers on at least one court and the HUD, and that an edited PNG in the mod
  folder shows up in the port.
- [ ] **P19 — HUD.** Score display, names, serve indicator, in-match menus using the game's HUD textures
  (`AZUMA/INPANE`) and layout. Verify layout and update frames against frame-stepped screenshots of the original.
- [~] **P20 — Audio.** Bank formats, driver slots and the play call mapped; note → tone unresolved (IOP driver).
  Journal `context/artifacts/2026-10-06-characters/1-MODELS-ANIM-AUDIO-PART.md`. Original text: HD/BD sound banks (Sony VAG/ADPCM), sound effects and their triggers (hits, bounces,
  crowd, umpire voice), MIDI BGM with the game's banks. Verify each effect's trigger frame against recorded game events, and decoded
  samples against PCSX2 audio captures.
- [ ] **P21 — Menus and modes.** (Includes the random-bounce option 0x2ef7e2 → `0x379ae0`, see net-hit journal 5.) Title, character/court select, exhibition, tournament/challenge modes, unlocks,
  options, save data. Verify menu flow, options and unlock conditions against the original screen by screen.
- [ ] **P22 — Widescreen, high frame rate, input polish.** Render-side improvements that never change the 60 Hz
  simulation; rebindable controls; controller hot-plug. Verify P0's full replay suite still passes unchanged with every
  option on.

## Known gaps / caveats

- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only); the net
  response shares the exact code path but its sweep is our flat net, not the court mesh (→ P15).
- `libm::sinf` ports only |x| ≤ 2^7·π/2 (asserts beyond); the game's callers stay in [0, π].
- Disc court folder ↔ physics court index mapping is unverified (slot 5 = court index 10).

## Needs the human

- (none yet)
