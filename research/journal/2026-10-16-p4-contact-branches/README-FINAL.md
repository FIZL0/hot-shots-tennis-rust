# P4 — the last contact branches: dive slide, per-character reach, arm-IK step

The open parts of P4: the dive/reach branch, the skeleton-derived values for every character, and the arm-IK sideways
step.

## Arm-IK sideways step

Already ported in N1d3b (the contact solve's body step and arm quats, `pose.rs`/`swing.rs`, tests in `motion.rs`).
No 0.1 m stand-in is left anywhere, so nothing to do here.

## Dive branch: the contact dive's slide

The dive search (frame, contact or miss, direction, the miss's lunge) was already ported and matched against 62
recorded dives. The open part was a *contact* dive's slide (+0x3f80), which needs the character's dive arm. Before
this change it was computed from zeroed shoulder/tip stand-ins.

- **The dive arm** (`pose::dive_arm`). The game builds it once at player load (in the same load step as the stroke
  arm table and the serve arms), from the receive motion (0x1e) at time 5. It holds two model matrices: the racket's
  at +0x3060 (a chain walk from the racket's own local) and the right upper arm's at +0x30a0 (identity walk, then its
  local). Oracle test `serve_ik.rs::dive_slide` compares them with RAM for all 20 players of the five RAM images
  (s03/s04/s05/s08/s09): bit-exact.
- **The slide** (`swing::arm_slide`). The game copies the player matrix (+0x3d40) with row 3 replaced by (0,0,0,1)
  and multiplies both arm matrices by it. The shoulder is the upper arm's row 3, the tip is the racket matrix applied
  to (0, 0.7, 0, 1). The ball is taken relative to the matrix's position (+0x3d70), with y negated. If the ball is out
  of reach, the slide is the horizontal distance minus the arm's reach across the line along the dive direction.
  Operation order (mula/madda/madd, the adda-from-zero steps) is taken from the disassembly. Oracle: 12,000 calls
  (random balls, directions, players, and both dive-shaped and arbitrary matrices) bit-exact against the game's
  function.
- **The matrix at the call.** Right before the call the game writes row 2 = the dive direction and row 0 = (row 1 ×
  row 2) × hand (+0x12b4), and leaves row 1 (0,1,0,0) and the position as they are. `dive_matrix` builds the same,
  including the signed zeros (the y and w of the side row).
- **The ball has y up.** First attempt: I passed the ball object's position (y down) and got 1.0/1.25 against the
  recorded 1.589/1.661. With rows 0 and 2 having no y, flipping the up row is the same as not flipping the ball's y,
  and that reproduced both recordings. So the game's predicted-path table holds the ball with y up, and the
  function's `× −1` brings it into the arm's y-down model space. `dive()` passes `[x, −y, z]`.
- **The direction's normalization** now follows the game's order (`mula 0,0; madda dx,dx; madd dz,dz`, then 1/√).
  With the recorded flight as the path, the same 38 of 62 dives give a bit-exact direction either way. The rest are
  1–6 ulps off, because the recorded ball drifts from the game's predicted table by ulps after a bounce (the
  test's existing 1e-5 tolerance). Kept the disassembled order.
- `recorded_dives` now checks the slide of the two recorded contact dives with the character's own arm (any of its
  costumes' skeletons):
  - p7e_c03 vsync 6595 (Kaito; its direction is bit-exact): the slide is bit-exact (1.661471).
  - p7b_c11 vsync 7654: 1.5893782 against 1.5893774. Its direction is 2 ulps off too, so the gap comes from the path,
    not the slide; checked within 1e-5.

## Skeleton-derived values for every player

`play/skeleton_reach.rs` (new, called once per player at setup after the mod data is in place) fills each player's
`Reach` from its own `CharacterData`:

- `ahead`, `body_low`, `body_high`: the stroke arm table (`pose::arm_table`, +0x3050/+0x3054/+0x3058), checked
  per RAM player by `motion.rs::arm_table_ram`
- `smash_ahead`: the smash arm's racket point z at frame 8 (`serve::ServeArm::point`, new; checked bit-exact
  against +0x38f0 for every RAM player in `serve_ik.rs::check_table`)
- `dive_arm`: as above

A mod character gets these from its own motions through the same code. The AI's chase goal (`b[2] − end × ahead`)
now reads its own player's `ahead`, as the game reads its player's +0x3050. Before, everyone used character 0's.
The hard-coded character-0 numbers in `reach()` stay as the base that each player's own values replace.

## Two match tests that depended on the old shared reach

With each player's own reach, bot matches play out differently (4-bot doubles, characters 0–3: 37 shots in the
first minute, 26 before), so two tests that leaned on the old timeline failed. Neither failure was a bug:

- `doubles_ai::path_cache_keeps_the_match` asserted `Game::shots > 0` at tick 3600. `shots` counts the current
  rally and goes back to 0 at each point, so the check only said a rally was under way at that tick. It now counts
  every shot struck over the run.
- `snapshot::receiver_gives_up_a_net_touched_serve` waited for a net-touched serve from a fixed random walk of the
  human's pads. The old seed's serve no longer comes. Seed 6's comes at tick ~16860, and the receiver gives it up
  as before (seeds 3 and 4 do too).

## Not verified / not 1:1

- **Characters in none of the five RAM images (s03–s09)**: their arm table, smash point and
  dive arm are checked only through the shared code path that is bit-exact on the characters that do have one. No
  per-character RAM capture of theirs exists, and these values are fixed at load, so a new capture would only
  repeat the same function.
- **Contact dive p7b_c11 vsync 7654**: 8 ulps off (1e-5 tolerance). The recorded ball path differs from the game's
  predicted table by ulps after a bounce (its direction is off too). Bit-exact needs the predicted table itself,
  which the live recordings don't hold.
- **Only two recorded contact dives** (p7e_c03, p7b_c11). The slide function itself is oracle-checked over 12k
  inputs, so that is coverage of the call site, not of the arithmetic.
- **The AI goal with a non-zero character** (`ahead` per player) has no match recording against it here. It's the
  game's own read of +0x3050, and the stock replay tests stay as they were.
