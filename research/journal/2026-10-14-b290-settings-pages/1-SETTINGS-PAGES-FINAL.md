# B290 — Settings menu rework (FINAL)

Settings is now a list of pages, every option row in one style (`play/main_menu/options.rs`):

- **Settings**: Display · Graphics · Sound · Controls · Online & Mods · Back
- **Display**: Display (fullscreen/window, not in the browser) · Aspect Ratio · Show FPS
- **Graphics**: Preset · Resolution · Shadows · Anti-aliasing · Frame Limit · VSync · Original Mode (everything a preset
  sets) · Upscaled Textures (locked off in the browser)
- **Sound**: Master · Music · Sound Effects · Voices (sliders) · Menu Music · Match Music
- **Online & Mods**: Upscaled Downloads (locked off in the browser). B289's online mod downloads setting goes here.

Every option row: label pill on the left, value centred between `<` `>` (tap the halves to step), sliders as a
gold bar; ◀ ▶ change, ✕ changes too (not on a slider), Back lands on the page's Settings row. The rows share one
top/pitch (46 / 39.5), so Graphics' nine rows end above the description bar. `Opt` (one enum) replaces the old
row-index `graphics_step`/`graphics_value` and the Settings switches; settings.txt keys are unchanged.
`HST_MENU=display|mod-settings` and `HST_PAUSE_SETTINGS=display` open the new pages. Legend lists per page.

Checked: `options.rs` tests (every page: walk, change, undo, Back row; sliders; presets/switches), pointer and pause
tests updated; `--shot`s of each page at 16:9, 1600×600 and 600×900 (in `context/b290/`). wasm32 `cargo check` ok.

## Not verified / not 1:1
- Menu is ours by design (front-end needn't match the original), so no comparison against the game.
- Controls page left as it was: B291 reworks it.
- Not run in the browser build; the wasm-only rows (no Display row, locked upscaled rows) were only compiled.
