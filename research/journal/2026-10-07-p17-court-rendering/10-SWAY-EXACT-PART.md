# P17m — costume sway, exact (in progress)

Done (`crates/hst/src/noise.rs`, `character.rs`):
- `Motion::sets` counts every `Motion::set`, a restart of the same motion included; the deformers reset on it.
- The noise offset d of each position entry is carried through its bone's posed matrix (Σ R·d, as the game moves the
  entry in bone space before skinning). Bevy skins the bind-pose mesh, so `draw` (Update, after `animate`) writes the
  bind-space offset (Σ w·R·bind⁻¹)⁻¹ · Σ R·d. At rest this equals the old bind-rotated offset.

Next: the VU check against a GS dump (Shift+F8 = GSDumpSingleFrame in copy 4's ini; `research/tools/gsdump.py`).
- Deformer object (vtable 0x1d0cf8): +0x1c phase, +0x20 rate, +0x30 freq, +0x34 prev, +0x40 amp; `**(obj+0x14)` is the
  NOI entry (+0xc rate, +0x10 amp), re-read every frame (step 0x150700, wind at 0x1bb5a8).
- Plan: poke every entry's rate to 0 (phase freezes; prev = phase, or 0 after a motion set) and its amp ×~200 so
  the sway is many pixels; GS-dump a frame; read prev/amp over PINE. Fit a DLT per bone (all of the character's
  packets share the frame's bone matrices) with the noise entries at `hst_sim::noise::deform`'s p'; the residual
  must stay at the 12.4 fixed-point level, and grow with p' = p or a wrong lane order.
