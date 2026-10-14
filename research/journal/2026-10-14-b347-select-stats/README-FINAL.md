# B347 — △ opens the character's stats on the new select

**Cause.** The B296 redesign of the character select (`chars` in `play/main_menu.rs`) dropped the old
select's `Player::card` toggle and its `attribute_card` call. The Press field `card` (△ / the Lob key) still reached
`Menu::step`, but the Chars screen ignored it. `attribute_card` was only used by the mods screen.

**Fix.**
- `Player::card` is back. △ toggles it on the Chars screen for the player that device drives, whether ready or not.
  It stays on across characters, as on the old select. On Random it does nothing because the pick is hidden.
- The player's big card draws the original's attribute card in place of the 3D render: tinted title box, play style,
  Serve…Footwork labels and A–F letters from `CharacterSelect2_04`. It is scaled to fit the card's height and up to
  60% of its width. `attribute_card` now takes a top-left corner and a scale (the mods screen passes 1.0).
  `grades_of` gives a disc character's MENU-overlay grades or a mod's TParam grades.
- A small Stats / Picture pill on the render's foot does the same for touch and mouse.
- The legend lists △ Stats / Picture.
- `HST_MENU=attrs` / `mods-attrs` open with 1P's card up again. Before this they were accepted but did nothing.
- Test `triangle_shows_the_attribute_card`. `legend_lists_what_each_screen_handles` still passes with △ listed.

## Not verified / not 1:1
- No screenshot of the new card layout: this unattended shell had no Wayland/Hyprland session
  (`HYPRLAND_INSTANCE_SIGNATURE` unset, no socket in /run/user/1000), so `tools/shot.sh` couldn't run.
  To check: `HST_MENU=attrs` plus `chars4`-like 4-player widths at 4:3 and portrait. The scale formula keeps the card
  inside the player card, but on a 4-player 4:3 row it is about 0.54× (small letters).
- The placement (inside our card, in place of the render) is ours. The original drew it over its own panel. The menus
  are not 1:1 by rule.
