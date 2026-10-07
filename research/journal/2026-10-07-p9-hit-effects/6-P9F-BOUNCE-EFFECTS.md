# P9f — bounce effects

**Result.** `hst_sim::effect::Bounce` is bit-exact against the game on every live frame of `bounce_s05`: 1440 frames, 8 bounces. That covers:
- the court marks: matrix, life, length and alpha;
- the dust puffs: position, life, size and alpha;
- the ballbound ring: start frame, world matrix, clocks and every morph weight.

The test is `crates/hst-sim/tests/bounce.rs`; the recorder is `tools/record_bounce.py`, and the decoder notes are in `research/bounce_rec.py`.

**The game's rules** (the character-effect manager's bounce object):

*Contacts and the first bounce*
- The ball keeps contact records of 0x50 bytes each: point +0x10, post-bounce velocity +0x20, normal +0x30, material byte +0x40. The bounce count is kept on the ball.
- On the first bounce of a flight the basis is built from the velocity:
  - fwd = horizontal velocity scaled by `div(1, sqrt(vx²+vz²))`;
  - up = (0,1,0);
  - side = up × fwd.
- On a court material, point.y is zeroed.

*What a court bounce makes*
- A non-smash court bounce adds a mark: at most 20, oldest dropped, lifetime 1800, length |v_xz|, drawn at y −0.01.
- It also adds a puff: at most 2, size 0.15, growth `div(0.3,20)`.
- A smash landing (≥ 85 km/h) starts the crater model `smash_bnd/cNN/cNN_chakudan` instead, plus one dust puff that drifts forward at 0.05, decaying ×0.95.
- The ring starts unless the crater is live. Its matrix is `rot_x(atan2f(−n.y, |n_xz|)) · rot_y(atan2f(n.x, n.z))`, placed at the point.
- The second bounce adds a smaller puff (0.1, growth `div(0.2,20)`) at the second contact.

*Per frame*
- Marks fade only while the fading flag is set: alpha = `div(life_before << 7, 1800)`.
- Puffs skip the frame they are made. After that they grow, fade by `div(128,20)` and live 20 frames.
- Puffs run only on courts whose look-table puffs flag is set; courts 0 and 7 have none.

**The court look table** is `exe::Game::bounce_looks`, 13 rows. Each row holds a dust RGB, a mark RGB and the puffs flag.

**Frozen frames.** The game sometimes stalls for a few vsyncs: ball, bounce object and models are all unchanged, and no update runs. The test skips frames identical to the one before. Before that skip, a mark's life looked off by one at frame 897.

**Capture.** A 2400-frame recording stalled at 1440 frames; the recorder spun on its re-grab loop. The capture was cut there, so the count check is ≥ 8 marks and rings. The recorder now reads the contact record in the same PINE batch and only warns when a read crosses a vsync.

**App.** `effects::BallBounce` (a resource) is fed by `start_effects` from the port's bounce count and landing, and drawn by `draw_bounce`:
- the ring and crater are posed like the impacts;
- marks are a cap/body/cap quad strip from `ballkon_00`;
- puffs and dust are camera-facing `kemuri` billboards in the court colours.

`--stage` picks the crater; c10 is the fallback outside 01..11.

**Not done.**
- wet courts (spray, puffs rising 0.01);
- the replay scale (1.5);
- slow-motion interpolation;
- an extra the bounce path calls for some shots;
- the game's blend modes.

**Screenshots** (`HST_AUTOPLAY=1 hst <iso> --play --stage 1 --shot … --shot-at t`):
- `context/shots/p9/p9f_9.png` and `p9f_20.png`: pale bounce marks on the clay; at 20 s a fading puff behind the ball.
- `p9f_5.2.png`: just after the first serve bounce.
