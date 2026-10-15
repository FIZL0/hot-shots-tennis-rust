# B461 Roster: dashed outlines get their corners

`roster.rs` `dashed()` drew only straight dashes from `r` to `len - r` (the last one clipped, so a side could end up
to a gap short of the corner) and nothing at the 14px corners.

Now:
- each corner is an `r`×`r` quad in the rim colour rounded on its outer corner, with an `(r - t)` quad in the card's
  fill colour inset `t` on top (no border support in the quad → UI node path, so rim-minus-fill); `dashed()` takes
  the fill (`hex(BG)` for an unjoined seat, `s.bg` for a computer's card);
- each side's dashes are spread so the first and last touch the corner arcs (gap stretched slightly to fit).

Checked by eye with `HST_MENU=chars` (unjoined 2P–4P) and `HST_MENU=roster1` (COM 2P's yellow rim) shots in
`context/B461/`: arcs drawn, dashes meet them. The COM card's top corners sit under the header band as before.

## Not verified / not 1:1
- Front-end design is ours (PLAN.md); no original to compare against.
- The corner arcs are solid, not dashed along the curve (14px arcs are shorter than one dash).
