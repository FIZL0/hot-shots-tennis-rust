# B318 — main menu "1v1 on this phone" label overflowed its pill

The touch-only Duo row's name was "1v1 on this phone": at 24 px in `word.tm2` (~12 px a glyph) it is ~200 wide,
the whole 200-wide pill (`LIST_W`), so it ran under the chosen row's `>` and past the pill's end.

- Renamed to **"1v1 Here"** (`home.rs` `name`); the preview still explains it ("Two players, one phone ...").
  Avoided "Pass & Play": the disc font may not have `&` (missing glyphs draw as a blank).
- Every list pill's name now goes through `fit_text` with max `LIST_W - 56` (20 px left inset, clear of the `>` at
  `LIST_W - 28`): a longer name later is squeezed across, never past the pill. Stock names are narrower than that,
  so they draw unchanged.
- Any aspect: the list lives in the 640-wide menu space scaled uniformly (`columns`), so the fit holds at every
  window size.

## Not verified / not 1:1
- Not looked at on screen: the Duo row only shows on a touch browser page (`touch_page()` is false on desktop),
  so no `--shot` of it; the fit is by the width arithmetic above. Front-end, not 1:1 by design.
