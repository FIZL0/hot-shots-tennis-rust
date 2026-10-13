# B29a — the serve's contact IK (body step and arm turns over the swing countdown)

Split from B29 (`../2026-10-12-b29-serve`). Captures: `context/b29/run3/` (`tools/record_serve.py`; slot 3 Carol
right-handed at x 3, z −12.25; S5 = slot 5 bots).

## The original

At the serve swing press (the human's and the bots' press handlers) the game picks the path point, stores it as the
contact (+0x3fb0, its height flipped back to the ball's −y), sets the swing motion, then solves:
- strong/weak toss → the overhead solve with the serve table (the smash uses the same solve with its own table);
- underhand toss → the underhand solve (no step, body never moves, only the arm turns).

Both tables are built from the swing motion posed at frame 8 (`serve::serve_arm`): racket point from the shoulder,
turned about x into the y–z plane, then the arm turned 135° about z (overhead) / 90° about y (underhand). The
overhead solve gives a body step (z first: behind the shoulder or past R/√2, both scaled by the side scale and the
second size factor) and a jump: the height the racket can't reach, as a scale of the swing's **root path**
(`*_serve_t_dummy`, `*_s_n_dummy` for the smash: y −0.23 at frame 8), not of the swing clip's root. Then the same
CCD as the strokes' `contact_solve` (z axis overhead, y underhand).

The IK's frames: n from 0 at the press tick (weight 0), frames = the countdown k (no `ik_frames` mapping), weight
n/k, the step through the mover each tick, full at the contact tick; then the strokes' ramp-down (8 of 8, n stops at
12). D: step x +0.0018936, x 3.0 → 3.0018935 over 12 ticks.

## Port

- `hst-sim/src/serve.rs`: `ServeArm`, `serve_arm` (table, lift from the root path), `overhead_solve` (step, jump
  scale), `underhand_solve`, the shared CCD.
- `character.rs`: the two dummy root paths kept in `paths` under the motions that use them (0x25 serve, 0x1f smash).
- `play.rs`: `serve_ik_start` at the swing press (`serve_turn`'s `Some(k)`), `serve_ik_tick` once per serve tick
  after the serve's own logic (press tick at weight 0, contact tick after the hit); the ramp-down then runs in
  `advance_stroke` as a stroke's. Mover without the partner check (the original skips it for the server).

## Verified

- `hst-oracle/tests/serve_ik.rs` `serve_ik`: the three tables (smash, serve, underhand) bit-exact against every
  player in s03/s04/s05/s08/s09; 12000 random solves against the original's two solve routines (step, the four
  joint quats, jump scale): 7669 with a step, 3031 with a jump, 0 differ. The first version drew contacts below the
  shoulder only, so no jump happened and the wrong lift (the swing clip's root, −0.913) passed; the captures caught it.
- `serve_ik_captures`: 5 recorded serves (A strong k 17, C underhand k 20, D early k 12, two slot 5 bot serves):
  solve from the recorded contact/position gives the recorded step and jump scale, and the per-tick x/z, n and
  frames match all 86 samples from press through the ramp-down, bit for bit.
- `play/serve_check.rs` `early_serve_steps`: D's press timing through the app (`serve_turn`): x per tick equals
  D_early's 13 samples bit for bit.
