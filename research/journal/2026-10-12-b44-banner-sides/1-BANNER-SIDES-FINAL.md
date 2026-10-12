# B44 — mph readout and finish banners against the screen sides

(a) Landed earlier as B44a (0ca23a1): the serve/smash speed readout sits on a full-window root and each quad goes
through `widescreen::pin`, so it keeps its distance from its own window edge at any aspect (`popups.rs`
`speed_tests::pinned_to_its_side`).

(b) Where the original draws the other banners (P12b4's decompile notes, 2026-10-07-p12b-popups/4):
- Finish word (Service / Return / Smash Ace, Counter): x 208, w 224 → centre 320.
- On the Line: x + 24 (232..456), riding under the word, the same in the original.
- Set / Match Point: x 192, w 256, scaled about its centre → centre 320.
- Game / Set result banner and the board (B93): centred too.

All of them are centred on the 640 screen, not side elements, so they stay on the centred 4:3 screen
(`widescreen::screen_43`), which is what B17's rule says for anything that isn't at a side. No move needed. New test
`banner_tests::centred` locks the finish word and Set / Match Point to x centre 320.

## Not verified / not 1:1
- No PCSX2 screenshot of a finish banner this task: the positions are the decompile's constants (P12b4), which
  are fixed numbers with no aspect dependence; triggering an ace on demand in the original would need a long
  vpad-driven rally. Rendering only has to look right, and the constants leave nothing to guess.
