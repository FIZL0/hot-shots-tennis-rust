# B533 — main menu Back returns to the press-start splash

- `Menu::step`, Screen::Main: ○ / Esc (and the title tab, whose hit already sends Back) calls `Menu::splash_back`
  (`main_menu/splash.rs`): 1P (`Menu::p1`) cleared, `Splash.claimed` reset, screen Press, so the next press takes 1P
  again. The backdrop's clock carries on. ○ on the Update row keeps its own meaning (Later).
- The keyboard always gets through (B516's gate only stops pads that aren't 1P); another pad's ○ does nothing.
- The `<` is drawn on the main menu's title tab (not while the mods restart ask is up); the legend lists Back.
- Test: `splash::tests::back_frees_1p` (pad 1P, other pad's ○ ignored, keyboard back frees 1P, another pad takes it,
  title tab tap goes back too). Shot: `context/B533/main.png`.

## Not verified / not 1:1
- none (front-end is ours; no original behaviour to match).
