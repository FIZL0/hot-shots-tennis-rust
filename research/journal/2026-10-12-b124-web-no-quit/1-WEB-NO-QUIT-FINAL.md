# B124 — no Quit on the web main menu

`MAIN` in `play/main_menu.rs` is cfg'd: wasm32 gets `["Local", "Online", "Settings"]`. Cursor wrap (`rows(MAIN.len())`),
the drawn rows and their touch hit areas, and the info line all come from `MAIN`, so they follow; on web the
`_ => Quit` arm can't be reached (sel < 3). Desktop keeps Quit.

Other `AppExit`s on web: the pause menu's quit (`play/menu.rs`) and an online match's end (`netplay.rs`) both run
inside a mode with a `back` (`main_menu.rs` Launch, `online.rs`, `main.rs` net), so `mode::back` turns them into a
switch to the menu while the window is open. The rest (`mash.rs`, `auto_shot`, preview bake/stress) are CLI/dev
paths (bake is already skipped on wasm). `web.rs` needed no change.

Checked: WebGL2 build (`target-wasm`, wasm-bindgen into web/pkg) driven with `tools/web_check.mjs … webgl2 55`
(screens in `context/b124/run1/`): visit 2 at 30 s shows the Main Menu with Local, Online, Settings only.
`tools/check.sh -p hst` passes.

## Not verified / not 1:1
- The pause menu's quit-to-menu on web wasn't driven in the browser (needs a match started and paused via touch/keys
  in headless chromium); reasoned from `mode::back`, which is the same code on desktop where it is exercised.
- Menu isn't 1:1 with the original by design (remaster-only menu).
