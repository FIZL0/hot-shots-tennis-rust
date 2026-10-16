# B475 — one Emote hint on the character select

- `main_menu/legend.rs`: the select's own-preview hints are one `(Act::Emote, "Emote")`; `Act::Preview` ("Win / Loss")
  is gone from `hst-play/src/play/legend.rs` (its R / F caps with it).
- `Glyph::Stick` is now one variant: the R cap with arrows all four ways (76 px wide, as the old left / right one).
- Keyboard (user 2026-10-16): R / F no longer play the preview's win / loss in the roster (`previews::buttons` reads
  pads only); the Emote hint's caps are the emote keys alone ("1-4"). A pad's right stick is unchanged (up / down
  win / loss, left / right emotes).
- Test: `select_keys` checks there is one Emote/Win hint and its keyboard caps are 1-4, and the glyph.
- Shots (`HST_MENU=chars`, `HST_LEGEND=xbox|keys`) in `context/B475/`: the Xbox bar shows the four-way stick + "Emote",
  the keyboard bar "1-4 Emote".

## Not verified / not 1:1
- None: this is front-end design (PLAN: menus are ours). I didn't compare it to the original, which has no such hint.
