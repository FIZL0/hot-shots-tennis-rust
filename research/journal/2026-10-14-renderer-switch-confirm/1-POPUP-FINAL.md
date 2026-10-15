# B327 — Renderer switch asks first

Settings → Display → Renderer (browser build only) used to save on each ◀ ▶ and the page reloaded 1.5 s after the
last step. Now a press on the row opens a confirm popup in the mods-change warning's style
(`mods_screen::ask_layout`: dimmed page, brown box, two pills, the hand):

- **Switch to WebGPU?** / *The page reloads to change it.* with ◀ ▶ around the name (keys, or a tap on either half
  of the title) stepping the offered renderer, never offering the one in use; **Switch** / **Cancel** pills, ↑ ↓
  between them, ✕ chooses, ○ or a tap off the popup is Cancel. Legend: Move · Choose · Cancel.
- Switch sets the setting and saves (`Effect::SaveSettings`, which calls `web::renderer`); Cancel changes nothing.
- `web/index.html` `hstRenderer`: reloads 100 ms after the save instead of 1.5 s (the debounce was only there for
  stepping past values, which the popup now does).
- The Renderer's side-panel choices were Off / On (the `choices` fallback); now Auto / WebGPU / WebGL2.
- No description-bar line (`said`): Settings has no bar and its ticker ran over the list's "Changes apply right
  away" footnote.
- `HST_MENU=renderer` opens Display with the popup up (for `--shot`).

Code: `options.rs` (`open_switch`, `switch_step`, `switch_layout`, test `renderer_asks_first`), `Menu::switch`, one
intercept line in `Menu::step`, `pointer::choose`, `legend::acts`.

## Checked
- Native `--shot` at 1280×720 and 540×960 (`context/b327/popup*.png`), next to the mods popup (`reloadmods.png`).
- Browser build (`WEB_FAST=1 tools/web.sh`), headless chromium (`tools/web_check.mjs`, `context/b327/web1/`): down to
  Settings, → opened "Switch to WebGPU?", Cancel kept Auto with no reload; → → Enter switched to WebGL2 and the page
  reloaded with `hst: WebGL2 (renderer setting: webgl2)`.
- Unit test drives the whole popup (keys and taps, the legend, its quad count within the pause overlay's pool at five
  aspects).

## Not verified / not 1:1
- Not 1:1 with the original: it has no such setting (front-end, ours to design).
- Not tried in a real browser with WebGPU hardware or on a phone: headless chromium only, where WebGPU falls back to
  WebGL2 before the first frame.
- From the in-match pause overlay Switch reloads the page too, so the match ends. The popup's line says the page
  reloads but doesn't mention the match.
- `mods_screen::tests::upload_all` failed twice when the whole main_menu set ran (packing threads late under load,
  its own comment says so) and passed alone. It doesn't touch this change.
