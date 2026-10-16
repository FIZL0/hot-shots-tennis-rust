# B464 — iOS Safari: Copy Link in the online lobby does nothing

## Cause
`bevy::clipboard::Clipboard::set_text` on wasm spawns `navigator.clipboard.writeText` and returns `Ok` at once. iOS
Safari only lets a page write the clipboard inside a tap's own event handler; the game reads taps in its frame
(requestAnimationFrame), so the write is refused (a console warning) while the lobby still said "Link copied".

## Fix
- `web.rs` `copy_sheet(text)` → the page's `hstCopySheet`. On a touch screen (`hstTouched`, touch.js) the page shows a
  card over the game and the touch pad (`#copysheet`, index.html): the link (or LAN code) in a read-only field, **Copy**
  (selects the field, `execCommand('copy')`, else `navigator.clipboard.writeText`, both inside the button's own click;
  where both fail the text stays selected: "Select and copy it"), **Share** (`navigator.share({url})`, only where the
  browser has it: iOS/Android) and **Done**; a tap beside the card closes it too. Clicks in the first 400 ms are
  ignored so the canvas tap that asked for the card can't close it with its own late click.
- `online.rs` `copy()`: the sheet first in the browser, else the clipboard as before; the lobby's status line says
  "Tap Copy or Share on the link's card …" instead of claiming it was copied. Desktop browsers (mouse) and native
  builds keep the direct clipboard write.

## Check
`node tools/web_copy_sheet.mjs [DIR]` — headless Chromium with touch emulation, the card cut out of index.html with
touch.js: 16/16 PASS (mouse page → game copies; after a touch the card shows and survives the opening tap; Copy puts
the link on the clipboard and closes; Done / backdrop close; Share shown only with `navigator.share` and handed the
URL; card on screen and centred in portrait and landscape). Screenshots in `context/B464/run1/`. wasm32 `cargo check`
clean for the touched files.

## Not verified / not 1:1
- Not tried on a real iPhone (no device in an unattended run): Chromium's touch emulation stands in. WebKit's
  "copy only in a user gesture" rule is met by doing the copy in the button's own click handler, which is the
  documented way, but that's untested on Safari itself.
- Front-end only (online lobby is ours, not the original's): no 1:1 comparison applies.
