# B123 — Menu Music / Match Music

The single Music switch is now two Settings rows (`play/main_menu.rs`). **Menu Music** is `menu_music` and defaults to on. It gates the menu BGM in `bgm()`, which follows it live because it watches `menu.settings` every frame. **Match Music** is `music` and defaults to off. It only controls `--music` in `Settings::flags()` for the match process, so toggling it never touches the menu track. An old `settings.txt` with `music = on` still means the match. With no `menu_music` line, the menu track plays (default on).

Tests: `settings_round_trip` covers a fresh file, an old `music` line, `menu_music = off`, and that `--music` follows only the match switch.
Shots (`context/b123/settings_{wide,43}.png`): 7 rows fit above the info bar at 16:9 and 4:3. `audio.rs` needed no change.

## Not verified / not 1:1
- Didn't listen to the audio. That menu music plays and court music doesn't comes from the logic tests and the `bgm()` gate, not an audio capture (an unattended run has no way to listen).
- Comparing with the original isn't applicable: the settings screen is our own design (PLAN, menus not 1:1).
- The Widescreen description is cut off at 4:3. That was already the case before this change and isn't part of B123.
