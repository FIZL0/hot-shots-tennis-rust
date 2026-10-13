# B152 — dark outline on menu titles

The original's confirm screen (PCSX2 shot `context/b40e/c4_start.png`, 1699 px wide = 2.65×) draws its titles
("Confirm Settings", "Umpire Lily", "Nakagawa River Courts", "Clay Court", play styles) in their fill over a dark ring:
3–4 shot px wide (≈1.5 px on the 640×448 screen), darkest ≈35–60 of 255 (the upscale blurs it; gold words' ring is
the darkest).

Port: `Draw::outlined(f)` (`play/main_menu.rs`) takes the font quads `f` draws and stamps them first in `RING`
(20/128 ≈ 40/255) eight ways round at 1.5 px, then the fill. The glyphs are the vector-rebuilt `word` sheet at
`menu_art::K`×, so the ring stays sharp at any window size; offsets are in 640 space, so it scales with the screen
and follows each quad's anchor (wide windows).

Wrapped: every screen's tab title (`frame`, keeps its soft shadow), the confirm screen's Umpire/name, court name,
surface, play styles and the 1P/COM tags; the online lobby's "Name - Singles/Doubles" header and court name, the
short code, "Getting the code…" and the missing-mod line.

Checked: `HST_MENU=confirm` shots at 1280×720 and 1600×600 (`context/b152/`), every outlined word legible on the
green grid.

## Not verified / not 1:1
- The lobby screen itself wasn't shot (it needs a live iroh host); same helper, same words, same wall.
- Ring by eye from one upscaled PCSX2 shot (rendering rule): eight stamped copies, not a true stroke, so very large
  sizes (the 46-px short code) show a slightly faceted ring.
- Text on pills/cards (buttons, seat rows, rank pills) left as it was: it sits on its own fill, not the wall.
