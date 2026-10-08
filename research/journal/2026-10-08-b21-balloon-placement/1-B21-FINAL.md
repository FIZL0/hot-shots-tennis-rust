# B21 timing balloons placed and sized like the original; upset swirl added (2026-10-08)

## Method
- Wrote `research/balloon_anchor.py`, a PINE logger. Usage: `balloon_anchor.py <slot> <vsyncs>`.
  - Finds the pop-up manager by its vtable word.
  - Logs each player's anchor node and the pop-up list: count at +0x6f0, 0x30-byte entries from +0x150;
    kind +0x29, sprite +0x1c, stage +0x14, timer +0x10, alpha +0x18, player +0x20.
- Read the draw function (393aa0) and the message handler (394ca0) in Ghidra.

## Findings
- **Anchor.** For pop-up kinds 0, 1, 6 and 7 the anchor is the player's Bip01Neck world translation
  (node index 8 for pc00, 6 for pc05), raised by 0.5 m; that point is the quad's bottom edge.
  The neck is about 1 m up. The port used a fixed head height of 1.77 m, which is why balloons drew too high.
- **Size.** Camera-facing square of side 2s, s = s0 * max(1, s1*z*t) * min(1, s2*z*t); z is view depth, t = tan(10 deg).
- **Per-kind table** at 0x4111d0 (stride 0x18):

  | kind | use | s0 | s1 | s2 | frames in, hold, out |
  |---|---|---|---|---|---|
  | 0 | timing balloon | 0.3 | 0.15 | 0.3 | 3, 45, 3 |
  | 1 | sweat / dots | 0.3 | 0.15 | 0.3 | 3, 60, 3 |
  | 6 | swirl | 0.35 | 0.15 | 0.3 | 3, 60, 3 |
  | 7 | "!" | 0.3 | 0.15 | 0.3 | 3, 30, 3 |

  Kinds 2-5 are other pop-ups: s0 0.3/0.3/0.45/0.3, frames 0,20,5 / 0,30,5 / 3,22,5 / 0,23,5.
- **Timing.** Balloon alpha runs 43, 86, then 48 frames at 128, then 85, 42, 0: 53 frames in all.
  P10's version gave only 47 opaque frames.
- **Upset pop-up = kind 6 swirl** (e_guruguru.tm2), message 0x17.
  - Fires in singles only, on point calls 1, 3, 5 and 6.
  - Goes over the last hitter, or over the server if no shot was hit.
  - Removes that player's "!" and timing balloon first.
  - The cell advances every 3rd tick and wraps after 5 cells. The original's UVs address an 80 px cell on a
    512x128 GS page; on the port's 400x80 texture that is u = cell*0.2, width 0.2, v 0 to 1.
  - Message 0x18 clears it. The port clears it at the serve.
- **Draw order.** Balloons and pop-ups use markers' `depth_bias` (LAST), so they draw after everything else
  in the transparent bucket.

## Port changes
- hst-sim `serve.rs`: `balloon_alpha` follows the original's ageing; new test `balloon_fade`.
- hst-sim `surprise.rs`: `Popup::Swirl`, `size()`, the swirl cell, `swirl()` (who gets it); tests.
- hst `play/surprise.rs`: anchor is the neck via the rig joint chain; `half_width` uses the kind's size; new
  `place_balloons` system places the balloon; the swirl spawns and its UVs are set; depth_bias.
- `play.rs` `balloons`: placement moved out, depth_bias added.
- `markers.rs`: `LAST` is now `pub(super)`.

## Verification
- Unit tests pass.
- Port screenshot with a balloon pinned for the test (context/b21/hack_5.png) against the original's
  context/b21/orig1.png: the balloon's bottom sits just above the head in both. Balloon height relative to
  player height is 0.37 in the port and 0.35 in the original.
- Not checked visually: the swirl. In singles P1 is human and waits to serve, so no point ends in an unattended
  run. The swirl logic is covered by unit tests (`sweat_who`, `swirl_cells`).
