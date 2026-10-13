# B184a — ceremony faces, and the air-guitar racket

User report: Carol's winners'-ceremony air guitar (`win_06_n`) had a still face and the racket in the wrong place.

- **Faces**: the ceremony motions ship `.MOR` (morph weights) and `.UVA` (eye UV) next to each `.ANI2`. `ceremony.rs` now
  loads them per player and slot with `character::face` and drives them like a match motion:
  weights at `t·80`, and UV offsets through `inspect::face_uv`. `CharacterData` keeps morph names
  (`morph_names`, `morph_target`) instead of a count, so a ceremony track can find its target.
- **Racket**: in the decompile's result screen, a per-player flag says whether the motion has a node named
  `Racket02`. If it does, the racket is placed on that node instead of the hand's `Racket` joint, still mirrored in x
  by hand. Motions with `Racket02`: `win_06_n`, `win_11_n`, every `lost_*`, `hand_00l/r_n`, `hand_05l/r_n`. The port
  dropped that node (`Clip::new` only binds skeleton names), so the racket stayed glued to the right hand.
  - ANI2 files are flat. The parent comes from the depth-first track order: after `Bip01LFinger42` it is the left hand
    (Carol, Will); after the toes or `Camera01.Target` it is the model root (lost motions). This is marked `ponytail:`.
  - `free_racket` builds a one-node clip (positions as authored). `pose` writes the matching local transform into
    the `Racket` joint.
- Checked by eye (`context/b184a/r9.png`, `r11.png`): Carol now holds the racket by the grip in her left hand like a guitar neck, and her eyes
  close and she grins mid-riff. No PCSX2 capture of the original's Carol win was taken. The loser's dropped racket
  (root-parented `Racket02`) is untested: in `--chars 0,6`, the shot at 9 s frames the winner.
