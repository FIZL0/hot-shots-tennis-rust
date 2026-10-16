# B481 — COM cards show "✕ to join" when a pad could take the seat

Done. On the local roster, a computer player's card gets a translucent dark bar across its preview and stats card,
thin gold lines on its top and bottom edges, and "<✕> to join" in the middle. The button is drawn in the
unseated pad's own art (Ⓐ on Xbox, ✕ on PlayStation, B on Switch, …). The bar's opacity pulses between 70% and full
over 2.4 s.

- `roster.rs`: `join_glyph(m, p)` decides which cards get a bar and which art they show. A pad's ✕ takes the seat
  `Menu::join_seat` picks: seats held open first, then computer seats in seat order. So the first k computer cards
  in that order get a bar, where k is the number of unseated pads, and each card shows its own pad's art (in pad
  order). There is no bar when no pad is unseated, off the local roster (online), or when `can_join` is false.
  `join_bar` draws the bar. Its height is 24% of the preview, at least 40 units. The button and words scale down
  to fit the card's width. The bar is drawn after the stats and Ready! and before the grid, so it covers the
  3D figure.
- `legend.rs`: `unseated_styles(seated)` returns the art of each connected pad that isn't in a seat. If
  `HST_LEGEND` names a pad style and no pad is plugged in, it returns one pad of that style (for `--shot`s), the
  same way `join_styles` does.
- Shots (in `context/b481/`): `HST_MENU=coms` with `HST_LEGEND=xbox` at 1280×720, `ps` at 900×900 and `switch` at
  600×1000 all lay out correctly. With no `HST_LEGEND` and no pad, there is no bar.

## Not verified / not 1:1
- There is nothing to match in the original: this is a front-end addition, and front-end screens are ours to
  design.
- I only checked by `--shot` with a forced `HST_LEGEND` style. I didn't plug in a real pad and watch the bar go
  away when it joins. The logic for that is the same `join_seat`/seat check that `step` uses.
- The keyboard is left out: an unseated keyboard (when 1P is on a pad) can join with Enter, but it doesn't show the
  bar. This follows B434's open-seat hints, which also don't show Enter.
- I didn't check the pulse in motion. A `--shot` freezes one frame.
