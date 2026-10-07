# N4a — the yellow smash marker against the original

## Capture
`research/smash_mark_rec.py 4 3000 context/n4a/r1` under `tools/pcsx2.sh`, with `tools/vpad.py` pressing ✕ every
1.5 s. It loads slot 4 (P1 is the only human; marker `+400` table: `[6, -1, -1, -1]`). Each frame it sets the
opponents' (slots 1, 3) pressed button `+0x3ee4` to △ while their swing countdown `+0x3ec4` is ≥ 0, so their
strokes become lobs. It logs the marker object (match manager `+0xa4`) and dumps the predicted path when the first
point is placed.
- One placement in 3000 frames: an opponent's lob launched at vsync 7956, point (2.4534, −8.5658).
- P1's heights come from the character table (+0xe8/+0xec): top 2.6, middle 2.2.
- Fixture: `context/fixtures/smash_mark_s04.txt`. Screenshots: `context/shots/n4a/orig_*.png`, F8 every 6 frames
  from the placement.

## Findings
- Growth: the path is 15 entries on the launch frame (the launch state plus 14, built from the hit message's
  direct call to the update), and grows 15 a frame after that: 30, 45, 60, 75.
  - **The original doesn't grow it a second time on the launch frame.** The port struck in `control`/`simulate`
    and grew the path in `start_effects` on the same fixed tick, so it was one frame early.
  - Fixed: `Marks::fresh` skips the growth on the strike's tick and still searches.
- Search: on the 5th frame (path 75) it placed `path[68]`, with the search's next index at 69. Fed the same way,
  `SmashSearch` gives the same frame and the same point, bit for bit (`tests/effect.rs smash_mark_s04`).
- Hide: the count `+0x144` drops to 0 at vsync 8039, when the live ball bounces, as ported. Path and red go at the
  next point (8159).
- After two predicted bounces, the original still grows the path while `len − frames since launch < 15`. The port
  stops. Nothing visible reads the path after the search is done, so this isn't ported.
- On screen: `orig_01` (placement) shows a faint yellow blob fading in. `orig_03` (about 12 frames on) shows a
  bright yellow soft blob with a light ring, wider than the red marker and between the aim and the net.
  - Port shots `context/shots/n4a/app_8.2.png` and `app_8.5.png` used a temporary hack, not committed: every
    receiver counted and the heights were 1.0/0.8, so ordinary bot shots place a point.
  - The port shows the same fade-in, then the same bright blob and ring, at a similar size against the red marker.

## Not verified
- The port's own predicted path against the original's. The live flight is verified elsewhere; the lob's launch
  spin wasn't captured here.
- Two humans' midpoint (doubles with two humans on one team) and practice's side pick.
