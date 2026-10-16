# M98 — promo screenshots: HUD-off mode, one set for README and site

## What landed
- `crates/hst-play/src/play/no_hud.rs`: `no_hud::OFF` (`--no-hud`, F10 toggles it). While a match runs, it hides every UI
  root (score panel, pop-ups, hints, name tags, pause/result screens), timing balloons, the 1P..4P/COM markers and the
  umpire's call words/banners. It puts each root's own visibility back when the HUD returns. It touches drawing only
  (PostUpdate, before visibility propagation), never the sim. The menus always show.
  The umpire's call models get their own system because `effects::pose` sets their roots to `Visible` every frame, which
  overrides a hidden parent (hiding `CallSpace` didn't work). `popups::Calls::roots()` exposes them.
- `main.rs`: `--no-hud`; `HST_KEYS` can press `F10`.
- `tools/shot.sh`: `HST_WAIT` (default 60 s); the header documents `--no-hud`, `HST_SIZE` and several frames per run.
- `docs/screenshots/`: every existing file refreshed (same names), plus `hero`, `cast`, `closeup-{sunset,autumn,indoor,beach}`,
  `mod-{castle,character,amusement}` and `phone`. JPEG q85, metadata stripped, ~5.8 MB total.
- README *Screenshots*: hero on top, then a 2-column table linking all 20 files.
- Site: `web.sh` already copies `docs/screenshots/*.jpg` to `web/shots/`, so the site uses the same files:
  - the band uses `hero.jpg`, or `phone.jpg` on portrait screens up to 760 px wide;
  - a new `.shots` gallery under About (sunset, beach, indoor, autumn);
  - the Mods figure uses `mod-castle.jpg`;
  - the Donate figure uses `cast.jpg`.

  Every path in README, index.html and site.css was checked to exist.

## How the shots were made
`context/m98/burst.sh DIR FROM TO STEP [hst args]` plays one `--no-hud --play` match and saves frames FROM..TO s to
`DIR/tNNN.png` through `HST_KEYS`. Settings for every run: `HST_WEATHER=0`, CPUs on both sides (`--pads -,-…`).
`HST_SIZE` sets the shape:
- default: 1280×720 → 2048×1152;
- `"1200 675"`: 1920×1080 (menus);
- `"1280 640"`: 2:1;
- `"450 800"`: 720×1280 portrait.

Runs (all in `context/m98/`):

| Run | Court | Characters |
|---|---|---|
| r1 | grass (05) | 0,1 |
| r2 | autumn (11) | 2,3 |
| r3 | indoor (07) | 4,5 |
| r4 | beach (02) | 7,8 |
| r5 | 04 | doubles 9–12 |
| r6 | 10 | 13,6, portrait |
| r7 | mod court `gag_c08_castle` | mods Mika + Ban, 2:1 |
| r8 | 06 | doubles of mods Jasmine, Chaos, Rosetta, Logan |
| r9 | mod court `gag_c07_amusement` | 6,9 with mod Sasha |
| r10 | grass | 12,13 |
| r11 | 08 | 9,10 |
| r12 | sunset (09) | 11,12 |

The best frames were picked by eye. `cast.jpg` is a 4×4 grid of crops: all 14 stock characters plus the mods Jasmine
and Sasha. Menu shots are in `context/m98/menus/`.

F10 was checked live: `context/m98/f10/strip.jpg` shows HUD on at 20 s, F10 → off at 23 s (a cutaway, no UI),
F10 → on at 26 s (every panel back).

Tests: `tools/check.sh -p hst-play popups` passed.

## Not verified / not 1:1
- No instant replay triggered in the sampled bursts, so the close-ups are post-point cutaways and celebrations, not
  replay angles. No smash or dive close-ups were caught. Forcing them would take more burst runs, but the cutaways
  already show the characters up close.
- `serve.jpg` uses the normal court camera, because there is no serve close-up camera.
- The crowd's "CLAP" words stay in HUD-off shots. They are an in-scene crowd effect, not HUD.
- Store thumbnails in `menu-browsemods.jpg` show letters, because the store's items have no thumbnails.
- The site layout was not opened in a browser at every width. The CSS was checked by reading only.
- Not compared to the original: this is our renderer's presentation (front end / rendering), which needs no proof by
  the rules. HUD-off doesn't touch the sim RNG or timing (drawing only).
