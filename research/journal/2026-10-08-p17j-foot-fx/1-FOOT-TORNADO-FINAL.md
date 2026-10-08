# P17j: footstep puffs, footprints, ball wind tornado — FINAL

## Footsteps (`hst-sim/src/foot.rs`, `hst/src/play/foot_fx.rs`; test `hst-sim/tests/foot.rs`)
- Run object in the game's manager; each tick it takes every player's right/left toe joint, the motion, the
  player's state byte (+0x3fa5: 1 running motions 3–7, 2 swings 16–31, else 0) and the court's foot table.
- Step rule: a toe dropping onto the ground while running/swinging spawns a puff (dry: `run/kemuri_00` in the
  court's dust colour; rain: `run/spray`) and, on dusty courts, a footprint (`run/e_footprint`).
- Court flags from the exe (`exe::Game::foot`): courts 1–6, 8–11 dusty with prints; 0, 7, 12 none.
  Prints: per court rgb + alpha/lifetime dry vs wet (court 10: rgb 25,22,20; alpha 128/89; life 80/120).
- Bit-exact against `context/fixtures/foot_s05.bin` (`tools/record_foot.py 5`, bot doubles, slot 5).

## Ball wind tornado (`hst-sim/src/tornado.rs`, `hst/src/play/tornado.rs`; test `hst-sim/tests/tornado.rs`)
- Object in the manager (update + draw functions in the retail exe; see `exe.rs::tornado_fade`).
- A struck shot (shot id not 999/9999) is latched; it starts on the frame the game flags the ball away
  (4 frames after the hit on a serve). speed = |ball vel|, scale grows by speed a frame up to 8·speed, then
  fades over 30 frames (alpha (fade<<7)/30); off below 90 km/h and at the first bounce. Matrix =
  rot_x(pitch)·rot_y(yaw) of the velocity, at the ball. UVA restarted at speed + 1.5 frames a frame.
- Model `wind2/tatumakiball` (EFFCT.XB0): 5 additive @vert materials, no MTA (effects::model now accepts that).
- Sim bit-exact: 2400 frames, 22 starts, 583 frames on.

## Ponytails / gaps (PLAN tasks P17o–P17q)
- Toes are the last drawn pose (one frame behind); state byte from motion id; motion sub-state 0.
- Puff wind drift 0 (weather wind not wired); puff camera push approximated (0.5 along view).
- Not ported: breath puffs, slide/dive marks and puffs, dash models, footstep sounds.
- Tornado: starts on the hit frame, not the latch; slow-motion interpolation and the flash quad not ported;
  its look (port: clearly visible white streaks; original screenshots: faint) not verified frame-matched.
- No wet-court capture test.

## P17o progress (2026-10-08, PART: usage limit hit mid-task)
- Done: puff wind drift from the weather wind. The game sets weather block +0x1a20 at each game as row 2 of
  rot_y(deg·π/180, wrapped ±π) × speed/60 (`hst_sim::weather::wind`). Bit-exact for 315°, speed 2 (0xbcc1165e, 0, 0x3cc11653).
- Found, not wired yet:
  - State byte +0x3fa5 is set by the mode setter. It is 0 stand, 1 run, 2 stroke/dive/whiff, 3 body hit. Port: 2 if
    `swing`/`dive`/`whiff`, else 1 if `body.running`, else 0. Footsteps only test ≠ 0.
  - The "sub-state" is player +0x3db0: the reaction motion. Port: `root.map_or(0, |r| r.motion)`.
  - The view row: 0x1e7f30 is camera→world (rows right, down, forward, eye). Row 2 is `g.cam.view.rot[2]`.
  - Toes: `character::animate` samples in Update. Factor the pose out and sample at `clock.sampled` in the tick.
  - Wet capture: message 0xe sets +0xa091 from weather byte gm+0x84→+0x135 (2/3). Record slot 5 with that byte
    poked to 2 each frame (`record_foot.py`). Start PCSX2 copy 3 again first (tools/pcsx2-hst.sh).
