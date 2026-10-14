# B289 — Online Mod Downloads setting

- Settings gets a row after Controls, **Online Mod Downloads**: Ask / Always / Never (←/→/✕ cycle, click works),
  saved in `settings.txt` as `online_mods = ask|always|never` (`Settings`' fifth field, `mods_online::Ask`).
- The lobby prompt reads it (`online.rs settle`) and its answer writes it (the old `Fx::Ask` → `mods-online/ask.txt`
  is gone; the row choice and the prompt share one value). A `settings.txt` without `online_mods` takes an older
  build's `mods-online/ask.txt` once.
- Settings now has ten rows, so its pitch is the Graphics screen's (35.5); the legend knows the new row.
- Tests: `settings_round_trip` (round-trip, unknown word → Ask), `settings_down_reaches_every_row` (ten rows fit
  above the description bar), `legend_lists_what_each_screen_handles`, `pack_unpack_checks_what_arrives` (old file
  read). Screenshot: `context/b289/settings.png`.

## Not verified / not 1:1
- Not compared with the original: a front-end screen of our own (no such setting in the game).
- Not driven through a live online lobby: the prompt path is a plain field read/write, covered by unit tests only.
