# C3e — readable presentation modules (camera, cutaway, shade, effect, sound, surprise)

C3 applied to the presentation modules of `hst-sim`: named constants, small modules of names and one enum in
place of raw offsets, packed numbers and byte flags. Behaviour-preserving only: every float operation and its
order is unchanged.

## What changed

- `cutaway.rs`
  - `ch` holds the channel numbers (pitch, yaw, roll, offset, framing, distance, fov, turn).
  - `op` holds the script ops: the framing shares 0..=4, the channels, the time/ease ops and `END`.
    `compile` and `intro_script` use these names.
  - `rec` holds the shot record's field offsets (frame numbers, flags, framing floats, band, heads, clearance).
  - `frame` names the reference frames; `post` names the post-point shots `pick` treats specially.
    `pick` also gets local name lists, and its loop order is unchanged.
  - Other names: `MAX_KEYS`, `DEG`/`HALF_DEG`, `camera::ASPECT` for 0.7, and `MAX_VFOV` (35°).
- `shade.rs` has `SCREEN`, `SHADE_RED`, `FULL`/`FOGGED_RED`, `COURT_HALF`, `CUTOUT_MODES` and `NO_KICK`.
  It uses `hst_data::mdl::prim::{TME, FGE}` and `Material::mode()`, two small additions in `hst-data`.
- `effect.rs`
  - `search::{RISING, FALLING, PLACED, GAVE_UP}` name the smash-search states. The field stays `i32` so the
    hashed Debug text is unchanged.
  - Shot kinds use `shot::kind`.
  - New constants: `TURN`, `BURST_SPEED`, `FIXED_PACE`, `SOFT_FOLLOWS`, `SMASH_COLOUR`, `SMASH_NET_GAP` and the
    sine table's steps per radian.
- `sound.rs` gets name modules: `slot` (sound slots), `court`/`racket`/`flight` (programs), `voice`, `umpire`,
  `reaction_motion`, `word` (umpire words) and `event` (gallery events). It also gets the volume, bounce and speed
  constants (the cheer volume is now `CHEER_VOLUME`), the `thud_key` helper, and `judge::verdict`/`shot::{branch,
  kind}` in place of raw numbers.
- `surprise.rs`
  - A private `Stage` enum replaces `stage: u8`. It is not hashed: pop-ups live in the rollback snapshot only.
  - New constants: `OPAQUE`, `SWIRL_CELLS` and `SWIRL_FRAMES`.
  - An `upset(call)` helper over `judge::verdict` replaces the duplicated match.
- `camera.rs` is unchanged. It already has named constants and the `View`/`Camera` types.

## Proof

- Baseline: `HST_NETPLAY=1 HST_HASH_TRACE=… tools/check.sh -p hst-play hash_trace -- --ignored`, once on the
  untouched HEAD in a detached worktree and once more with `HST_HASH_GAMES=3`.
- Result: both traces are byte-identical after the change (`cmp`; 10703 and 14713 lines, `context/c3e/`).
- `tools/check.sh -p hst-sim --lib --test cutaway shade effect sound camera intro replay_camera platforms trails
  sparks umpire` passed (85 tests).
- Float literals were checked against the `from_bits` values they replace.
  20° (0x3eb2b8c2) and cos 50° don't round-trip as decimal literals, so they keep `from_bits`.
- `cargo clippy` on the libs of hst-sim and hst-data gives no errors. The `--all-targets` errors (effect.rs test
  floats, serve.rs test 0.7071) were already there at HEAD.

## Not verified / not 1:1

- The names are inferred from how the code uses the values and from the existing docs, not from the original's
  symbols. This covers sound programs/keys and gallery events, cutaway record fields, frame and post-point shot
  numbers, and shade's mode range. They are labels only: behaviour is proven unchanged by the hash trace and tests.
- The doc of `ReplayPicks::shot` says "4 the △ smash", but the play side sends 4 for (class 3, kind 1) and 6 for
  a dive. The code is unchanged and the doc is left as is (a doc/behaviour question, not a C3 rename).
- shade.rs's receiver-pass test `pk.flags & 8` stays a number: the meaning of that VU1 object-flag bit isn't
  documented anywhere.
- The replay camera numbers in `ReplayPicks` (0x1d, 0x22, 0x2b…) stay numbers: nothing documents which shot each
  one is.
