# P17p: footstep extras

## Scope: what the game does in a match

The run object (char-effect manager +0x720) has a per-player trigger, 3993f0, gated on run+0x120. Per player it does: the sit-down burst check, footsteps, dive start and the dash-flag update. Puff aging (39a720) runs after all players.

## Puff slot (0x80 bytes)

0 type, 4 phase, 8 timer, 0xc size, 0x10 grow, 0x14 alpha, 0x18 fade, 0x20 pos, 0x30 dir, 0x40 foot, 0x44 player, 0x48 aim, 0x4c speed, 0x50 fresh, 0x60 vel (type 1), 0x70 scale. Reused slots keep stale fields. 100 slots; the oldest is dropped.

## Puff rows

Dusty 0x4114a0 and wet 0x4114d8. Fields: 0 fade_in, 4 fade_out, 8 fade_out2, 0xc alpha, 0x10 alpha2, 0x14 size, 0x18 grow, 0x1c speed, 0x20 rise, 0x24/0x28 ring_size, 0x2c/0x30 ring_speed, 0x34 ring_rise.

- Dusty: (3, 25, 25, 96, 48, .15, .3, .03, -.003, .1, .15, .01, .03, 0)
- Wet: (3, 10, 15, 96, 64, .3, .3, .03, .01, .15, .25, .01, .03, .014)

## Type 2 "burst"

Per-character flags at char table +0x10c (56 bytes): motion 47 for chars 0, 1, 4, 6, 8, 9, 10; 45 for 1 and 5; 46 for 13. These are the sit-down reactions after losing a point, not slides. It fires once (latch at run+0x9ec0+0x1dd+p) when pelvis y >= -0.25 (char 13 motion 47: spine1, y >= -0.26): 4 puffs at the pelvis, k>=1 pushed 0.2 along its z row, k1/k2 offset x by +-0.15.

## Type 1 "dive ring"

On the manager's hit event with branch 3 (dive), if the court is dusty or wet, 10 puffs go in a ring around the player. dir is the level unit vector player to Bip01Head (negated in rain; y stays +0, w becomes -0). Size and speed come from the sound manager's MT19937 (*(0x43b1d0)+0x740; P17p called it the shared one, P3e2 corrected it) (uniform = lo + (hi-lo)*2^-32*u); velocity steps by 36 degrees. In all 8 recorded dive frames (foot_s05x/w/d) the rings' 20 draws are the frame's only sound-generator draws, straight from the previous frame's state (P17p3): the run object takes the players' hit events the same frame, so in the app foot_fx ticks after `simulate`/`character::tick` and draws from `Game::rng.sound`; the test asserts it.

## Dash streak

When +0x3f80 (lunge) > 0.5 at the dive, the EFFCT.XB0 `run/dash` model is started at rot_y(atan2(m20,m22)) of the pelvis with its translation. It is cleared when the motion leaves 0x1e or +0x3ec9 is set; messages 0xe/0x19/0x1a clear it.

## Not in matches

- Debris particles (par/tuti dirt, par/siba/run/spray water; 3985d0/398c30) and the walking step sound 0x69 are gated on gm+0x344 (close-up/training only).
- Dive sounds were already ported (sound.rs).
- "Breath puffs" (iki_no_moto) belong to the court's carnon machine model, not the run object.

## Verification

tools/record_foot.py `extras` records the shared MT, run+0x9ec0 (0x1f0), per player +0x3f80, +0x3ec0 and the pelvis/spine1/head matrices. FOOT_POKE=dusty|wet forces run+0xa090 each frame so a hard court produces rings.

Test hst-sim/tests/foot.rs foot_extras_s05 runs over foot_s05x (5000 frames, slot 5), foot_s05w and foot_s05d (700 frames each, dive at frame 249). Every puff (all kinds), the latch and the dash matrix are bit-exact. It resyncs on recorder-missed frames and at point end/start (puffs jump there).
