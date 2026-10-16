# M97 — Donate page with dev stats (site + in game)

Already there from the earlier pass (ac6142c2): `tools/dev_stats.py` → `research/dev-stats.json` (ledger) and
`web/stats.json` (totals), run by `tools/release.sh` before it builds; the site's Donate section (`web/index.html`
`#donate`, `site.js` `stats()`) reads `stats.json` and links https://ko-fi.com/fizlo.

This pass: the in-game screen.

- Main menu gets an **Extras** row (after Settings). Its preview: a Ko-fi picture, one line of words, and the
  prompts / tokens / commits as fact tiles.
- ✕ opens **Extras** (`crates/hst/src/play/main_menu/extras.rs`, new `Screen::Extras`, title tab "Extras"): a list
  like the main screen's (only Donate so far) and the Donate panel: four big tiles (prompts, tokens, commits,
  sessions), the site's wording ("support my work", never the game), and Ko-fi / Since / Counted facts. ✕ opens
  the Ko-fi page (`modstore::open`: browser tab on the web, xdg-open/open/start on desktop); ○ goes back to the
  main menu's Extras row. Legend: Move / Open Ko-fi / Back.
- Stats are `include_str!`'d from `web/stats.json` at build time (release.sh commits a fresh one before building),
  parsed with serde_json; numbers written as the site does (`7.0 billion`, `1,585`).
- With both the Rematch and the Update cards up, five pills plus the Update card would overlap Quit: the pills'
  pitch drops 46 → 42 in that case only.
- `HST_MENU=extras`, `main-extras`, `rematch-update` (both cards) for `--shot`. Shots (in `context/m97/`): 16:9
  extras, main-extras, crowded; 4:3 extras — all lay out fine.
- Tests: `extras::tests` (stats parse + number format, Main → Extras → Back), legend probe covers `Screen::Extras`
  (the Ko-fi open is skipped under `cfg(test)` so the probe doesn't start a browser).

## Not verified / not 1:1
- Not 1:1 by design: the original has no Extras/Donate screen; front-end is ours (PLAN.md).
- Opening Ko-fi on the iOS/Android apps: `modstore::open` there falls to the desktop branch (xdg-open), which
  likely does nothing on a phone; the link is shown on screen (ko-fi.com/fizlo) so it can be typed. Not checked on a
  device (no phone here).
- Opening Ko-fi on desktop not clicked through (it would open a browser on the user's desktop in an unattended run).
- The stats in a build are those of the last `tools/dev_stats.py` run committed before it (release time), not live.
