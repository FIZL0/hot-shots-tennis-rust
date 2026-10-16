# M5b — Installed → Texture packs: on/off and order list

B373 had already put the kind tabs on every Mods page; M5b was left with the Texture packs tab of Installed.

## What it does
- `crates/hst/src/play/main_menu/mods_screen/packs.rs`: the installed packs (`mods_textures::ordered`) as one row a
  line in their saved order (top wins), each with its place, name, "by author - N textures - size", an On/Off switch
  and ▲/▼ buttons. Pad: ✕ switches, △ moves up, □ moves down (the cursor follows the pack), Select removes (the usual
  Remove child). Mouse/touch: the switch and arrows are their own hit rects (`Act::Row(slot, OK / card / page)`);
  the row body selects. At an end the move says so with the "no" sound.
- Each change is saved at once (`mods_textures::set_order(order_of(rows))` + `Effect::SaveSettings` →
  settings.txt `texture_packs` / `texture_packs_off`), marks the page changed (Reload offered) and says "Reload to
  see it" — textures are read at startup, as for installs.
- The side panel: name, author, count, size, an On/Off badge, the place ("On: 2nd of 3 in use" / "Off: not used"),
  the folder, and Turn on/off · Move up · Move down buttons. The legend lists Turn on/off, Move up, Move down, Remove.
- No Sort or Game filter pills on this tab (the order is the point; packs have no source game); a narrow (portrait)
  panel's strip drops its second line and the list takes the room (`grid::lay_of`).
- `hst-base/src/mods_textures.rs`: `ordered` (all packs in order, off ones in place), `order_of`, `bytes`;
  `arrange` is `ordered` filtered. `mods_kind::coming(Textures)` is now None (the tab is real).
- Tests: `mods_screen::tests::texture_packs_list` (order not A-Z, pad ✕/△/□ with cursor following and the end
  stop, saved order, legend words, taps on switch and ▲, hit rects on screen at 4:3/16:9/21:9/9:16),
  `kind_tabs_filter_every_page` updated, `mods_textures::tests::order_and_settings` extended. A `DRAWN` lock now
  serialises the tests that draw the grid and read `grid::ADD_AT` (`scrolls_and_taps` failed once when the new
  test drew at the same time — a pre-existing race on that static).

## Checked
- `--shot` (`HST_MENU=addmods HST_KIND=3`, three packs, one off via settings.txt) at 16:9, 4:3, 21:9 and 9:16
  portrait: `context/m5b/run1/packs*_*.png` — rows, long names, buttons, panel and legend lay out at every aspect,
  scaled uniformly.

## Not verified / not 1:1
- Not 1:1 with the original: it has no texture packs or mods screen; the screen is ours (PLAN: menus are ours).
- The live app wasn't driven by pad/mouse/touch input end to end; presses and taps are checked in unit tests
  (`Menu::mods_step` / `point` on the drawn hit rects), and settings.txt writing goes through the existing
  `SaveSettings` path checked by M5a and the options tests.
- Browser (WebGL2/OPFS) and phone builds not run for this list: same code path, not shot.
- Changes take hold on Reload, not live (textures are loaded at startup), by design.
