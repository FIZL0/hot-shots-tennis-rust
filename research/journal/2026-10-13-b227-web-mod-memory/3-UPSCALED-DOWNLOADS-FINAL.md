# B227 (3): Upscaled Downloads setting on desktop

User: "we should probably have a setting on desktop too to download upscaled or not, (defaults to whatever the
upscale textures setting is)".

- Settings gains **Upscaled Downloads** (settings.txt `upscaled_downloads`). If settings.txt doesn't have it, it
  takes the Upscaled Textures value, and turning Upscaled Textures on or off sets it to match. You can then change
  it on its own.
- On: store installs download the whole zip as before. Off: the install takes the browser's path (range requests,
  `textures/upscaled/` never fetched) and writes the rest to disk. The ignored test
  `ranged_install_leaves_the_upscaled_set_on_the_store` covers both the in-memory and on-disk installs (Anya: 35 of
  489 MB).
- In the browser, both Upscaled rows are locked off (shown OFF, they don't toggle, the description says why).
- Settings row pitch went from 44 to 39 px to fit 8 rows above the description bar. The pause menu's Settings
  uses the same pitch, and `rows_under_clicks` was updated. Screenshot: `context/b227/settings.png`.
- `tools/check.sh`: 512 passed.

## Not verified / not 1:1
- The Settings menu is our own design (no original to match).
- Turning Upscaled Downloads on later doesn't fetch the upscaled set for mods that are already installed. You
  have to reinstall the mod.
- Pause-menu Settings with the new pitch is not screenshotted. It is only checked by the `rows_under_clicks` test.
