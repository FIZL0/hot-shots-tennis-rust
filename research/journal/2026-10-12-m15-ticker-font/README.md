# M15 — crisp menu hint font, smooth ticker scroll

The description bar's text was drawn as `ascii.bmp` glyph quads (11×22 texels, whole 640-px steps). Now
(`play/main_menu.rs` `ticker_text`):

- One Bevy UI `Text` node in Fira Mono (Bevy's built-in `default_font`, OFL, no disc art), rasterised at the
  window's physical resolution. Monospaced, so the em is sized to make the advance (0.6 em) exactly the
  original's 11 px pitch on the 640-wide screen; tint 223 (the old `bar_font` ink), centred on the bar (y 404..436).
  The `ascii.bmp` load, `bar_font` and `Tex::Bar` are gone.
- The frame counter (`Ticker`, fractional 60 Hz frames) moved from `draw`'s `Local` to a resource; `ticker_glide`
  lerps between the original's whole-frame positions, but not across a wrap (that still jumps).
- Any aspect: the text sits in a clip node over the bar's `Anchor::Span` rect, so on a wide window the bar's text
  runs edge to edge; `ticker_x_in(f, n, right)` is the original's `ticker_x` with the entry/re-entry point at the
  window's right edge + 16 (656 on 4:3 / widescreen off, identical to before). Same 616 px test, 32 px/frame
  entry, 2 px/frame scroll, 160-frame hold.
- `HST_WINDOW="W H"` (main.rs) sets the window's physical size, for unattended shots at 4K etc.

Tests: `ticker_as_measured` unchanged and passing; `ticker_glides` (whole frames = 60 Hz positions, halves between,
no glide across the wrap, wide-bar entry/scroll/re-entry).

Shots (context/m15/): `s4k.png` 3840×2160, `s169.png` 1920×1080, `s219.png` 2520×1080, Options screen with the
widescreen line scrolling — sharp text, bar and text span the window.

## Not verified / not 1:1
- Letter shapes: Fira Mono is not the boot program's font (no licence-clean copy of it exists); it is thinner and
  its x-height differs. Chosen as the closest built-in monospace match; not compared side by side against a PCSX2
  screenshot this run (front-end is ours by PLAN.md, so judged by eye only on our own shots).
- On a wide window the loop length grows with the window (text re-enters at the window's right edge rather than at
  x 656 mid-window); at 4:3 / widescreen off it is the original's loop exactly.
- Narrower than 4:3: the font scales with width (keeps the 11-unit pitch), so it's smaller relative to the bar.
- Original Mode (P23a, HUD at 448 lines): not checked how the native-res text looks there.
