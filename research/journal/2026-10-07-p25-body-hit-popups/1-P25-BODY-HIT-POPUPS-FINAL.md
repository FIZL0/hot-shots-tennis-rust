# P25 — body-hit pop-ups (final)

## Trigger
Player rally update 0x349410 calls the body test 0x35b9d0. Each frame in the rally phase (gm+0x55 == 3), outside replays, every player in mode 0/1 (standing/moving; not swinging, mode 2) is tested against the ball's node position.

- Hit if the ball is within 0.25 m of (Bip01Head origin − head x-row·0.15).
- Otherwise hit if it is within r of the segment Bip01Spine→Bip01Neck, measured square to it (the projection must fall inside the segment).
- r = player+0x13ec, copied from the character record. That is TParam column 60's second half (`ReachStats::collision`), confirmed from the parser at 0x98xxx (fStack_c).
- Bone nodes player+0x17f0+4i are looked up by name strings at 0x41e088..: 0 Racket, 1 Bip01Head, 2 Bip01Neck, 3 Spine2, 4 Spine1, 5 Bip01Spine, ...

On a hit, 0x34afc0 mode 3 runs: motion 0x2b, message 0x14, DAT_0042305c = player, voice 0x3553d0(p,5). The judge ends the point in the same tick (phase 3→4 in the capture).

## Frequency
Once per point: pop-up object flag +0x73d and DAT_0042305c are reset at a new point (msg 0xe/0xc). There is no randomness; "sometimes" means whenever a body hit happens.

## Word
Pop-up object (vtable 0x1d1cc0), update 0x392400:

- CONK (`e_poko.tm2`, texture 7) if ball speed <= 50 km/h (0x375cb0: |v|·60·3600/1000).
- Otherwise SMACK (`e_bashi.tm2`, texture 8).

Both are 64x32, in `AZUMA/C_EFF/EFFCT.XB0` data/azuma/panel. WHIFF (e_suka) and FLOP (e_dosa, kind 5 dive landing) are other triggers and not part of P25.

## Animation
Kind 3, table 0x4111d0+3·0x18 = {0.45, 0.16, 0.3, fade-in 0, hold 30, fade-out 5}.

- Spawns at the ball with alpha 128 at once (no fade-in).
- Every tick for 20 ticks y −= 0.6/30 (rises 0.4 m).
- Scale +0.15 x10 to 1.5, then −0.05 x10 to 1.0, then 1.0.
- In a match (>= 2 players) it holds at full alpha until removed. Message 0x19 (players react) removes kind 3 instantly, and a new point clears everything. On a fault there is no reaction, so it lasts until the next serve.
- Practice (< 2 players): holds 30+1 ticks, then fades over 5 ticks (alpha 102, 76, 51, 25, 0).

## Draw (0x393aa0)
- Anchor = pos − cam_forward·0.5 − (0, 0.5/2, 0): 0.25 m up, 0.5 m toward the camera.
- Depth d from the view matrix; t = tan(fov/2) with fov 20° (DAT_001e7d50).
- size = 0.45·max(0.16·d·t, 1)·min(0.3·d·t, 1).
- Half-width = size·1.2·scale; height = 2·size·0.6·scale.
- The quad hangs upward on screen from the anchor along the camera's down row.
- Colour 128, alpha from the entry.

## Verification
`tools/record_bodyhit.py 5 context/p25/orig.bin context/p25` (slot 5 bot doubles, court with the ruins) forces DAT_0042305c = 0 30 frames into the rally and records the pop-up object every frame.

- Spawn tick: scale 0.15, n=1, already risen once.
- Scale 0.15...1.5 at tick 10, 1.0 at tick 20.
- y from −1.4388 to −1.8188 (−0.02/tick for 20 ticks).
- Alpha 128 held; removed at frame 84 when the next point started (the forced hit on the serve was a Fault for the server's team).
- Matches `hst_sim::bodyhit` tick for tick.

Screenshots via F8 land about 10 frames after the request, so only the held frame is pinned to a frame. At hold (frame 40) the quad from the formula, projected with the recorded camera, matches the original's SMACK within 1–2 px in place and size (`context/p25/compare_040.png`, original on top, prediction below).

Note: this PCSX2 copy's screenshots have a 4/3 wider horizontal projection (and about 1.08 vertical) than the game's own matrix (fitted from the court lines). Use that when comparing screenshots with the game's projection.

Port: `HST_AUTOPLAY=1 HST_BODY_HIT=0 hst <iso> --play --shot context/p25/port_4.0.png --shot-at 4` forces the same hit: "Fault after 31 frames" as the original, SMACK drawn above the ball.

## Port
- `crates/hst-sim/src/bodyhit.rs`: hit test, `Popup` word/tick/quad, tests.
- `crates/hst/src/play/bodyhit.rs`: per-tick detection from the posed bones before `simulate`, pop-up aging after it, the billboard.
- `play.rs`: `Game::body_hit` passed to `judge.check`/`judge`, and `react`'s `body_hit`.

## Left out (ponytail)
- The ball's own course after a body hit (it flies on).
- The hit player's immediate 0x2b motion and voice at the hit (only at the reaction).
- Mid-crossfade poses for the bone test.
- Alpha interpolation between ticks (only matters in practice fades).
- The practice-only message 0x15 re-arm.
