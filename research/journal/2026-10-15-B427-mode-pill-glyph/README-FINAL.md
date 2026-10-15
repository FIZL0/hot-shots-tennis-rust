# B427 — Singles / Doubles pill shows its button

The character select's top-right mode pill now draws the button that swaps the mode, left of its word, in the art
of 1P's device (the one `can_swap` / `online_swap` accept): Tab on the keyboard, □ on PlayStation, X on Xbox / Deck,
Y on Switch, the left dot on a generic pad. It reuses the legend's baked `Tex::Glyph(style, 4)` (the Mods page's □)
so it's the same art as the legend; `legend::dev_style` is now `pub(super)`. Shown only when the pill takes the swap
(local roster, or online host) and 1P has a keyboard or pad; the pill widens by the glyph and the tap area follows.

Shots (`HST_MENU=chars HST_LEGEND=keys|ps|xbox|switch tools/shot.sh`): `context/B427/all.png` (1280×720, three
styles), `context/B427/portrait-crop.png` (720×1280).

## Not verified / not 1:1
- none (front-end, ours to design; no original counterpart).
