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
- **Characters are original stand-in designs.** Players, the umpire and background NPCs are drawn with our own
  stand-in figures (`crates/hst/src/figure.rs`); their *behaviour* — states, timings, contact frames, positions,
  calls, reactions — is ported exactly from the game's code and animation timing data. Do not import or
  reproduce the original character models or character artwork.
- **Same input, same result.** Given the same controller input on the same frames from the same starting
  state, the port must produce the same hit spot, contact frame, timing grade, shot and ball path as the
  original. Prompt P0's input-replay harness is the acceptance test for every gameplay prompt.
- Record findings and evidence in `context/artifacts/<date>-<slug>/` (memory maps, decomp addresses, captures).
- Update **Done** and the **Play it now** controls when a prompt lands.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```
WASD/left stick move (and aim at contact) · Shift/LB sprint · J/A topspin · K/B slice · I/X flat · L/Y lob ·
U/RB drive · J/Space/A/Start serve · C/Select camera (follow/broadcast/free) · arrows/right stick turn camera.
`HST_AUTOPLAY=1` lets a bot play your side (unattended tests). `--stage 01..11` court, `--court 0..11` surface.

## Done (ported and verified against the original)
- Disc/XB/TIM2/MDL/MTL readers; court layout placement; Bevy renderer, 60 Hz fixed sim + interpolation.
- Ball flight (drag, Magnus, gravity, curve/bend), ground bounces, rolling — 4524 recorded frames.
- Shot tables (TRAJ): lookup + launch speed/elevation/frames — 11 recorded strokes.
- Per-character shot parameter records (17 per class/kind, record = character + 3) built from GAME.BIN —
  bit-identical to the game's runtime table, using the PS2 FPU model.
- `hst_sim::ps2` — PCSX2's EE FPU model: chop rounding, add/sub alignment with one guard bit, **div and sqrt
  round to nearest** (the emulator's divider mode), DAZ. Proven on 2912 table values and the ball integrator.
- Ball flight on PS2 arithmetic in the original's instruction order: every airborne frame of 39 recorded shots
  is bit-exact (2160 frames). Bounce frames still differ in the last bits → P0a.
- Net contact (flat net from the game's predictor), material-based bounce response.
- Stroke contact search + timing grades (SWEET SPOT / QUICK / SLOW) — ground-stroke branch.

## Prompts

### Harness
- [ ] **P0 — Input replay harness.** Record the original's controller state every frame over PINE (find the
  pad buffer the game reads in RAM) together with ball, player and match state, starting from a save state.
  Replay the same input sequence into the port from the matching starting state and diff every frame: contact
  frame, hit spot (ball position at contact), timing grade/offset, launch, ball path, player positions, score.
  Ship it as a test runner (`cargo test` on recorded fixtures + a CLI for new recordings). All later prompts
  are accepted only when their recordings replay identically.
- [ ] **P0a — Bit-exact bounce.** Port the contact path with the original instruction order so
  `crates/hst-sim/tests/flights.rs` can require bit-exact position *and* velocity on every frame (today: airborne
  frames exact, first miss at the first bounce). Pieces: plane sweep `0x328ed0` (contact skin 1.005×r, eps 0.005,
  0.98 factor, VU0 dot products), contact point `0x12fc30`, VU0 helpers (normalize `0x125b10` with Q sqrt/div,
  cross `0x125ac8`, mat×vec `0x125a50`/`0x125a80`, inverse `0x32d190`), quaternion slerp (`0x12d620`, `0x12d860`,
  `0x12d340`) and the game's own sin/cos/acos (`0x115fb8`, `0x115d18`, …). Needs a `vu0` arithmetic model
  (PCSX2 VU path: chop, no add alignment — verify like `ps2`). Asm: `context/ghidra_scripts/DumpAsm.java` →
  `context/notes/asm_*.txt`.

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
  Remove the approximate wind-up reach growth in `find_contact` by porting the exact reach term.
- [ ] **P4 — All contact branches.** Port every branch of the contact search in `0x34d8a0`: forehand/backhand
  ground strokes (body-shot variants `0x16/0x18/0x1a`), volleys, smash (1.85–2.65 m, reach 1.1), the
  reaching/diving shot (offset −10), half-volleys, and their per-character timing tables (`+0x1510`, `+0x1644`, …).
- [ ] **P5 — Shot selection by input.** How buttons, hold/charge time and stick map to class/kind/power and the
  charged blend; double-tap / combo inputs; special shots (`0x408e60` list). Verify with recorded inputs
  (capture pad state over PINE alongside shots).
- [ ] **P6 — Serve.** Real toss (`37af20`), serve tables `serv0..3`, serve timing/power meter if any, faults,
  service box rules, second serve, serve positions per side/court. **Lets:** a serve that clips the net cord
  and lands in the box is a let and is replayed; one that clips the cord and lands out is a fault — the cord
  contact must come from the exact net collision (P15), and the let/fault call and replay flow must match the
  original (verify with recorded let serves).

### Players
- [ ] **P7 — Player movement.** Exact run/sprint speeds, acceleration, turning, split-step, auto-positioning
  toward a locked contact, per character (TParam.csv + player object fields). Capture player position/velocity
  per frame over PINE from slots 3/5 and match frame-exact.
- [ ] **P8 — Player animation timing.** Port the player state machine and the animation timing from the game's
  ANI/ANI2/MOR data and code: idle, ready, run directions (`run_f/b/l/r`, `dush_f`), shot animations
  (`sh_*`), contact frames, recovery, celebrations/reactions. Drive the stand-in figure's poses from those exact
  states and frame timings (stand-in visuals, original timing).
- [ ] **P9 — Hit effects.** Hit flashes, ball trails, impact/bounce effects and their timing from `AZUMA/C_EFF`
  (EFFCT.XB0: `impact_*`, `ballbound_*`, `smash_*`, `chakudan`), including blend modes and UV animation (.UVA, .MTA).
- [ ] **P10 — Timing pop-ups as the original.** Use the game's own pop-up textures and animation (find the
  sprite/texture for SWEET SPOT / QUICK / SLOW-style feedback in `AZUMA/INPANE`, `CMN`, `MENU` archives and the
  code that animates it: scale/fade curves, position, duration) instead of our text pop-up.

### AI
- [ ] **P11 — Opponent AI.** Port target choice, shot type choice, positioning, reaction delay and timing error
  from the AI code and AIParam.csv per character; it reads the stored path (path recorder: 15 steps/frame,
  court plane only). Verify AI decisions against slot 5 (all-bot) recordings.

### Match
- [ ] **P12 — Rules and scoring.** Games, sets, deuce/advantage, tiebreak, side changes, doubles rules, match
  flow states (point start/end delays, replays if any) exactly as the original.
- [ ] **P13 — Umpire (Lily).** Port the umpire's behaviour: calls (score, fault, out, let, net) and their timing,
  chair placement per court, idle/turn reactions, voice-line triggers. Stand-in figure for visuals.
- [ ] **P14 — Background NPCs.** Spectators/ball kids/creatures from the layout's creature/gallery records
  (categories 21/23) and their animation timing (`azuma/gallery/ani/npcNN_*`, `trgCreMdl`) — positions,
  paths and triggers exact; stand-in figures for visuals.
- [ ] **P15 — Court collision mesh.** Live-ball collision against the court mesh (`FUN_0032f690` on `gm+0x84`):
  net cord/posts, walls, fences, other materials; replaces the flat net. Net-cord hits must behave exactly:
  balls that clip the cord and dribble over (rally net cords and serve lets), balls stopped by the net, post
  hits. Verify with live-ball traces (the stored path ignores the net, so record the live ball `*(gm+0x98)`).

### Presentation
- [ ] **P16 — Camera.** Port the original in-match cameras exactly: broadcast/follow angles, FOV, smoothing,
  serve/replay/point-end cameras (`camed/cam_cNN_*.dat`, `.CAM`), per court. Keep our free camera as an extra.
- [ ] **P17 — Court rendering fidelity.** Material blend modes and flags (MTL header), vertex colour/lighting,
  cloud placement (category 14 records), sky time-of-day variants, seasons (`_sXXXX`, `SSN1`), animated
  textures (MTA/UVA), shadows (incl. the per-frame shadow blobs), fog.
- [ ] **P18 — Upscaled textures.** Map the user's `replacements/` pack (PCSX2 hash-named PNGs) onto disc
  textures (compute PCSX2's texture hash from TIM2/MTI data + CLUT) and load them in place of the originals.
- [ ] **P19 — HUD.** Score display, names, serve indicator, in-match menus using the game's HUD textures
  (`AZUMA/INPANE`) and layout.
- [ ] **P20 — Audio.** HD/BD sound banks (Sony VAG/ADPCM), sound effects and their triggers (hits, bounces,
  crowd, umpire voice), MIDI BGM with the game's banks.
- [ ] **P21 — Menus and modes.** Title, character/court select, exhibition, tournament/challenge modes, unlocks,
  options, save data.
- [ ] **P22 — Widescreen, high frame rate, input polish.** Render-side improvements that never change the 60 Hz
  simulation; rebindable controls; controller hot-plug.

## Known gaps / caveats
- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only).
- Disc court folder ↔ physics court index mapping is unverified (slot 5 = court index 10).
