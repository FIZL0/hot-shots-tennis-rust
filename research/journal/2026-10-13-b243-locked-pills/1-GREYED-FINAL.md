# B243 — locked switches look disabled

The Settings screen (main menu and the pause menu's Settings, which draws through the same `drawn`) now draws
the rows `locked()` names (browser build: Upscaled Textures, Upscaled Downloads) with `Draw::rows_locked`
(`play/main_menu.rs`): a half-transparent dark pill, dark words, and never the chosen row's gold, even when the
hand is on it or a tap selects it. The row stays reachable (its description says why it's off), and tapping it
still does nothing, as before. Other screens call `rows_at`, which is `rows_locked` with nothing locked.

Check: `locked_rows_greyed` (a locked row under the hand has no gold fill; both rows keep their hit). By eye:
`locked()` temporarily forced on natively, `HST_MENU=settings` `tools/shot.sh` at 1280×720 and 720×1280
(`context/b243/settings_locked*.png`): both locked rows read as disabled at both aspects; reverted after.

## Not verified / not 1:1
- Not seen on a real phone or in the browser build (the native shot forced the lock; same drawing code).
- The front end is ours (PLAN), so there's no original to match.
- Seen in passing, not this task: in the 720×1280 shot, every menu label is squeezed sideways (the font isn't
  scaled evenly in a portrait window), on every row, locked or not.
