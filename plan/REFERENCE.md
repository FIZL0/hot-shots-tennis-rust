# Reference

## Controlling the real game (for recordings and checks)

- `tools/pcsx2-hst.sh` launches PCSX2 with PINE on; `tools/pine.py` reads/writes RAM and loads/saves states.
- What you can do with your PCSX2: `tools/pcsx2-hst.sh` start · `stop` · `status` (is it up); `tools/pine.py` (no args)
  prints the game id and running/paused, `tools/pine.py <hexaddr>...` reads RAM; pine tools resume a paused copy
  themselves; `tools/screenshot.sh out.png` takes a screenshot (a copy's own F8 hotkey); `tools/vpad.py` presses
  buttons. pcsx2-hst.sh takes no other words (they'd be opened as a file and block the copy with an error dialog).
- **Matches end.** A match finishes on "Game, Set, Match!" with "✕ Continue" and stays there; nothing after it is
  gameplay. Long captures (slot 5 runs a whole bot match) can run past it: before trusting a long capture, take a
  screenshot at its end, and cut or re-record what came after the match ended.
- **Save states (the user's — load only, scratch saves go to 8/9):** 3 = start of a game, P1 human + 3 bots;
  4 = mid-rally right after the serve; 5 = full bot game. Slot 5 has no human input — use it for ball/AI
  captures only. Controller-input recordings (P0 and anything gameplay-from-input) start from slot 3 or 4 with
  P1 driven by `tools/vpad.py`.
- `tools/vpad.py serve` creates a virtual Xbox-360 pad (uinput, no root); PCSX2 binds it as `SDL-0` when no
  real controller is connected. With `HST_PCSX2=N` (parallel runs) it is pad N, the only pad PCSX2 copy N sees. `tools/vpad.py send "press cross 120" "stick l -1 0" "sleep 300" release`.
  Never press Select (PCSX2 hotkeys are Select + shoulder combos). Timing is wall-clock; frame-exact replays
  should use PCSX2 input recording (`.p2m2`) — P0 decides.
- `tools/screenshot.sh out.png [pattern]` captures a window (default PCSX2) without focusing it; with `HST_PCSX2=N` it uses
  copy N's own F8 screenshot instead (copies sit on hidden workspaces, where grim sees nothing; full-size PNG; not while paused).
- `vpad.py send` returns at once and the server runs commands one after another (sleeps included): a script
  that sends several batches must wait each one out (`vpad()` in `tools/pick.py`), or presses pile up and land late.
  In menus ○ confirms, ✕ goes back.
- **Choosing characters:** `tools/pcsx2.sh python3 tools/pick.py <char 0–13> [slot]` loads save slot 2 (doubles
  character select), puts character <char> in as P1, takes the three COM picks where their cursors start, starts
  the match and saves P1's first serve to the scratch slot (8). It prints P1's character (+0x12bc), hand
  (+0x12b4) and model hand flag, and stops if the wrong character got picked. All 14 checked (P7b). Slot 2's P1 has
  the "switch hand" toggle on, so every pick plays with the hand opposite to TParam's. To serve from that slot,
  press ○ three times about 1.5 s apart: the first press after a load or a point is ignored, the second tosses,
  the third hits. P1 serves every point there and points are short (about 2 s).
  Not automated yet: choosing the COM characters (each COM cursor starts on a different character), the costume
  (L1), the switch-hand toggle, singles, P1 receiving.
- Verify input effects numerically over PINE (e.g. ball/player state), not by eye.
- `tools/overnight.sh` (in tmux) runs `claude continue` (TUI: attach to watch or type; it never asks) back to back and owns the virtual pad for the night.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```

Doubles by default (`--singles` for 1v1). Player 1 = keyboard + controller 1, player 2 (player 1's partner; the opponent in singles) = controller 2
when connected; the other slots are CPU. WASD/left stick/d-pad move (and aim at contact, screen-relative) ·
J/A (✕) topspin · K/B (○) slice · L/Y (△) lob — stick toward the net at contact: flat; pulled back with slice: drop · J/Space/A/Start serve ·
C/Select camera (original/free) · arrows/right stick turn the free camera.
Rebind any of these in `controls.txt` beside the ISO (written with the defaults on first run). Controllers can be
plugged and unplugged mid-match: each keeps its slot, a new one takes the first free slot. Wide windows show more court at the sides (the game's vertical view kept); the HUD stays on a centred 4:3 screen.
`HST_AUTOPLAY=1` makes every slot CPU (unattended tests). `--stage 01..11` court, `--court 0..11` surface. Drawing runs uncapped (the simulation stays a fixed 60 Hz tick, visuals blend the last two ticks); `--vsync` caps it to the display.
Gamepads whose device node is read-only (udev rules that strip write to stop rumble) work through the patched
`third_party/gilrs-core` (read-only fallback, no rumble).

## Done (ported and verified against the original)

- B24a: court 4's passing ball (trigger type 15): rolled once a match at a change of ends, route, animation, bounce sounds and fade bit-exact against a recording (route 0); drawn in play (journal 2026-10-08-b24-court-events).
- P2d: the rally aim returns its nudge (±5/±10), short-only flag and the smash's held depth (`shot::Aim`) into the launch; the smash launch gets the original's timing scatter (`swing::smash_scatter`/`smash_scale`); aim.rs checks all three per recorded aim and that ×0.6 would miss a sweet incoming slice in singles and doubles, timing.rs 28 smash launches bit for bit (journal 2026-10-08-p2d-aim-leftovers).
- B21: timing balloons sit just above the head (neck anchor) at the original's size and timing; singles players show the swirl on losing a point.
- P22: widescreen HUD on a centred 4:3 screen (play/widescreen.rs; the 3D view was already Hor+, F0 already uncapped), rebindable keys/pad buttons in `controls.txt` and hot-plug-stable controller slots (play/controls.rs); render/input only, hst-sim untouched, full test suite passes.
- B7: one-sided court materials back-face culled as VU1 does (MDL material +0x24 = 0; winding from the third vertex's UV w); court 1's near school fence no longer blocks the camera (mdl.rs, gs.rs; tests/winding.rs; journal 2026-10-07-b7-camera-blocked).
- P18: upscaled and moddable textures — PCSX2 pack hash names computed from the disc GS data (XXH3 of swizzled blocks + CLUT), lookup mods/textures → replacements/ → disc, `--dump-textures`, hot reload (hst-data texhash.rs, hst textures.rs; journal 2026-10-07-p18-textures/README.md).
- P17k: weather exact — schedule from the MT19937 seeded by a newlib `rand()` output (bit-exact vs slot 5), mirrored ripple tiles, rain voices sweeping ±45°, costume `.NOI` wind sway (EE phase bit-exact, VU noise from the microcode), particles under the court fog (journal 2026-10-07-p17-court-rendering/8-WEATHER-EXACT-FINAL.md).
- P17h: court model mipmaps — TEX1 MXL from MTL header +0xf, K from MDL material header +4, level ⌊log2(view depth)+K+½⌋ (journal 2026-10-07-p17-court-rendering/6-MIPMAPS-FINAL.md).
- P7c: reach and contact heights — `hst_sim::player::ReachStats::from_tparam` (the game's strtok/decimal parser) bit-exact against RAM records for all 14 characters (tests/player.rs reach_stats_from_tparam); play.rs gives each player its own contact-search reach.
- P17g: court shadows — sun direction bit-close to RAM on courts 04/10, strength 1 − (⌊0.xx·255⌋·255>>8)/128 multiplied on the hole ground (measured on the PS2 shot), casters = players + plant records with code byte 3 ≠ `'0'` (shadow.rs, gs.rs, gs_prepass.wgsl).
- P12b1: point score pop-up — player plates, score roll (old value squashed, new grows) + white flash, Deuce! banner with × count, from flow's score show; layout checked on screen against the original (play/popups.rs).
- P11e2: singles centre walk + net dash — `hst_sim::position::{zone, Single, Court}`, `play.rs` ai_wait_singles; test tests/singles.rs on context/fixtures/ai_pos_singles.bin.
- P11e1: doubles formation — each bot's lane, front flag and waiting spot (point start, re-pick on hits / every 30 frames, hold) and the centre-rate/radius walk back, exact on every change in ai_pos_s05 (position.rs, play.rs ai_wait).
- N4: landing markers — the game's `chakudan_p` (red, at the aim, from the serve return) and `smash_p` (yellow, smash-point search) models; red checked on screen against the original; N4a: yellow's placement frame and point bit-exact on a forced-lob rally (no path growth on the launch frame), fade-in and look match on screen (effects.rs, play.rs).
- N5a: △ smash (kind 1) — contact search, launch (smsh1, 5° spin) and flight to the first bounce exact on lob_smash_s05; one locked swing per team (swing.rs, shot_tables.rs).
- N3a: sound banks (`hst_data::snd`), note → tone and SPU pitch bit-exact vs save states (sound.rs).
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
- Post-point umpire call show (settle on voice end/countdown, hold, fade; out/double fault chain into the score show), human press ends the phase once the score settles, HUD old score until the roll — all 35 point-over phases of a recorded bot match and 8 human phases tick-exact (P12a).
  point-over phase of a recorded bot match tick-exact.
- Stroke contact search + timing grades (SWEET SPOT / QUICK / SLOW) — ground-stroke branch.
- F0 — uncapped frame rate: render-side blending of positions, facing, motion time, camera and balloon fades; sim unchanged.
- N1d1 exact ANI motion sampler (squad/Hermite, bone-length scale), bit-exact vs RAM.
- Motion clock (N1d2): per-frame motion time as the game's motion player — sample at the wrapped/clamped time, then add the speed; soft follow-through held by its 8-frame crossfade — bit-exact on 35026 recorded ticks (anim_s05.bin).
- Court collision mesh in play (P15): the ball always meets the disc court's mesh (net, cord, posts, walls, props);
  aimed live-ball recordings (post hits, cord dribble-overs in rallies and serve lets, cord/net stops) bit-exact.
- Shot buttons (P5a): ✕ topspin, ○ slice, △ lob only; the stick at contact turns topspin flat (within 60° of forward) and slice into a drop shot (within 45° of back; not on serves; smashes unchanged) — 42/42 recorded human aims in round1.bin.
- P14c2: ambient sound emitters (`npc::Emitter`: countdown, re-arm, pan bit, type-36 sweep) bit-exact on courts 10, 1, 2, 4; wired into play's court sounds.

## Known gaps / caveats

- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only); live-ball
  recordings do (`tests/live.rs`). The app without `--stage` collides with court 10's mesh (court drawn flat).
- `libm::sinf` ports only |x| ≤ 2^7·π/2 (asserts beyond); the game's callers stay in [0, π].
- Disc court folder ↔ physics court index mapping is unverified (slot 5 = court index 10).
