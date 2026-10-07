# Reference

## Controlling the real game (for recordings and checks)

- `tools/pcsx2-hst.sh` launches PCSX2 with PINE on; `tools/pine.py` reads/writes RAM and loads/saves states.
- **Save states (the user's — load only, scratch saves go to 8/9):** 3 = start of a game, P1 human + 3 bots;
  4 = mid-rally right after the serve; 5 = full bot game. Slot 5 has no human input — use it for ball/AI
  captures only. Controller-input recordings (P0 and anything gameplay-from-input) start from slot 3 or 4 with
  P1 driven by `tools/vpad.py`.
- `tools/vpad.py serve` creates a virtual Xbox-360 pad (uinput, no root); PCSX2 binds it as `SDL-0` when no
  real controller is connected. With `HST_PCSX2=N` (parallel runs) it is pad N, the only pad PCSX2 copy N sees. `tools/vpad.py send "press cross 120" "stick l -1 0" "sleep 300" release`.
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
J/A (✕) topspin · K/B (○) slice · L/Y (△) lob — stick toward the net at contact: flat; pulled back with slice: drop · J/Space/A/Start serve ·
C/Select camera (original/free) · arrows/right stick turn the free camera.
`HST_AUTOPLAY=1` makes every slot CPU (unattended tests). `--stage 01..11` court, `--court 0..11` surface. Drawing runs uncapped (the simulation stays a fixed 60 Hz tick, visuals blend the last two ticks); `--vsync` caps it to the display.
Gamepads whose device node is read-only (udev rules that strip write to stop rumble) work through the patched
`third_party/gilrs-core` (read-only fallback, no rumble).

## Done (ported and verified against the original)

- P17g: court shadows — sun direction bit-close to RAM on courts 04/10, strength 1 − (⌊0.xx·255⌋·255>>8)/128 multiplied on the hole ground (measured on the PS2 shot), casters = players + plant records with code byte 3 ≠ `'0'` (shadow.rs, gs.rs, gs_prepass.wgsl).
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
