# B309 — closed as obsolete (no code change)

The user decided on 2026-10-14 to take "1v1 on this phone" out of the mobile build entirely (B379, in `plan/TODO.md`).
B379 has already been built on branch `task/B379` (5f9fdf15), which isn't merged yet. It deletes everything this
bug is about: `widescreen::SIDEWAYS`, the quarter-turned HUD (`in_view_at`'s turn, `hud_gamma`'s turned extent and
uv, `--sideways`), the duo select's half turn, and the main menu's 1v1 Here row. Its TODO edit already marks B309
"obsolete: … nothing to do unless it comes back".

Fixing the loading cover's orientation (it isn't turned with the HUD, `loading.rs`) and the turned score HUD now
would be thrown away by B379 and would conflict with its deletions in `widescreen.rs` / `hud_gamma.rs`, so this
task changes no code.

If the feature comes back, the lead was: `loading.rs` builds the cover on the plain window and never looks at
`widescreen::turned`, while the HUD is laid out turned and `hud_gamma` turns it back; on a real phone the screen's
orientation lock (`web/touch.js`, portrait in fullscreen while duo) and the page's own orientation decide which
turn is right, so both need checking on the device, not in headless Chromium.

## Not verified / not 1:1
- Nothing was changed or checked on a phone: the feature is being removed (B379, user 2026-10-14).
