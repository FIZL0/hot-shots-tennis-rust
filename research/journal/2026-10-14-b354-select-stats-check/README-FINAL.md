# B354 — B347's △ stats card checked by eye

B347 was merged into main only, so this branch cherry-picked its commit first.

**Shots** (`context/b354/run2` before, `run4` after; `HST_MENU=attrs`, `mods-attrs`, `mods`, `chars4` through an
`HST_BIN` wrapper because `hyprctl exec` drops env; 960×720, 1280×720, 1680×720, portrait 720×960).

**What was off (run2).**
- The card was sized to the whole card height, so its title box sat under the "Right hander" pill and the Footwork row
  under the "Picture" pill, at every aspect.
- At 4:3 it was 60% of the card wide and covered "Keyboard" and the "Net Player" style line.
- At 4:3 a long name ("Phoebe") ran under the Stats / Picture pill. This also happened with the render up.

**Fix** (`play/main_menu.rs`).
- `card_scale` fits the card between the hand tag (top + 28) and the pill (bottom − 26), at most 55% of the card's width
  and never above 1.0. The card is centred in that band and right-aligned.
- The style line is clipped to the left of the card while the card is up.
- The name stops 58 px short of the right edge whenever the pill shows.

**After (run4).** Nothing overlaps at 4:3, 16:9 or 21:9. The card shows the original's tinted title box, play style
and the Serve…Footwork A–F letters, and it is the same `attribute_card` the old select / mods screen draw.

## Not verified / not 1:1
- Size: the card is about 0.6× the original's 144×160 on a 720-px-high window, because the player card's height
  limits it. It is readable, but smaller than the old select's full-panel card. Placing it inside our card is our
  design (menus aren't 1:1 by rule).
- Not shot: a 4-player portrait with the card up, because no `HST_MENU` opens that. `card_scale` caps it at 55% of
  the card width, so it fits by construction.
- Not driven: a live △ press in a window. The toggle is covered by `triangle_shows_the_attribute_card`, and the shots
  open with the card already up.
- With the picture up, "Baseline Player" still runs into the render at 4:3. This comes from B296's layout, not this
  card, and was left alone.
