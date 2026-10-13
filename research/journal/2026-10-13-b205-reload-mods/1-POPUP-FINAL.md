# B205 — ask to reload after a mod install/remove

- `mods_screen.rs`: `View.changed` (mods changed since start: the gold "Reload" note beside the main menu's Mods row,
  and its description) and `View.fresh` (changed since last asked). Set by a removal, a store/desktop install
  (`Event::Installed(Ok)`), and the browser picker's "Added …" status (index.html's word for a saved mod).
- Leaving the Mods page with `fresh` opens the popup on the main menu (`ask_step` takes every press while it's up:
  ↑↓←→ move, ✕ chooses, ○ = Later; taps on the pills via the page's `pointer`). Drawn by `ask_layout` in the
  pause dialog's colours (`menu.rs` PANEL/CHOSEN/UNCHOSEN, now `pub(super)`): dimmed menu, rounded box, two-line
  title, Reload/Later pills, hand; in the centred 640×448 frame with a full-window dim (any aspect ratio).
- Reload: browser `location.reload()` (`web::reload`); desktop starts `current_exe()` again with the same args and
  quits (the roster, mod textures and sounds are read at startup, so a rescan would miss pieces). The Mods page's
  Reload row now does the same on desktop (it used to only say "restart by hand").
- `HST_MENU=reloadmods` opens the main menu with the popup up. Shots at 1280×720 and 900×900:
  `context/b205/wide.png`, `square.png`.
- Test: `removes_on_the_second_press` extended (ask after removal, Later closes, note stays, no second ask).

## Not verified / not 1:1
- Front-end, ours by design (PLAN): no original counterpart.
- Desktop restart not driven end to end (needs a key press in a live window); `cargo check` for wasm32 passes but
  the browser reload wasn't run in a browser.
- The "Reload" note after Later wasn't screenshotted (only the popup).
