# N1e

- [x] **N1e — Motion blending (crossfade).** The motion setter `0x350280` calls `0x140130(motion, frames, id,
  flag)` before switching: `frames` < 2 cuts; otherwise the outgoing motion (id +0x44, data +0x48, time
  +0x50/+0x54, +0x58) is kept and faded out over `frames` (+0x5c, countdown +0x60). Lengths by caller: 8 for
  looping motions (stand/run/ready, the `param_4 != 0` path) and for the whiffs 0x27/0x28, the caller's value
  for 0x1c/0x1d, a cut for everything else; stand↔ready (1↔2) also sets +0x74 = 6. A switch mid-blend keeps
  whichever pose is more than half in (weight +0x60/+0x5c past 0.5) and rescales the countdown. Port the
  per-bone mix the ANI player does with the two poses (`0x140360`/`0x1404c0`, how +0x44.. is sampled and
  weighted: lerp/slerp, root handling) and replace the app's hard switches. Verify blend start, length and the
  mixed bone transforms per frame against a recording of the motion object (+0x44..+0x74) through
  stand→run→stroke→ready transitions.

Done: `hst_sim::motion::Fade` (start/tick/tick_hold) and `motion::mix`. The state machine is bit-exact against the anim_s05 fixture (35283 frames, 3076 fading, 713 sets of which 10 are held and 121 mid-fade, 0 missed). The per-bone mix is bit-exact against RAM (3 players mid-fade in s04/s08/s09, 69 tracks). It is wired into the app in `crates/hst/src/character.rs`.
