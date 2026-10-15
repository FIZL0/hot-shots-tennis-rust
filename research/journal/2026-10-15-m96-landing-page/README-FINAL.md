# M96 — OpenTennis landing page (concept v3–v5 built) — FINAL

Built from the concept's notes (`../2026-10-15-landing-page/README-READY.md`, v3 + v4/v5 changes); the claude.ai
artifact itself isn't reachable from here, so its look was rebuilt from those notes.

## What shipped
- `web/index.html`: opens OPFS first; no `disc.pack` (or `?site`) → the site (`web/site.js`, `web/site.css`) before any
  of the game's wasm is downloaded. Import (worker.js, progress bar) or Play resolves it and the old boot continues.
  The old plain ISO form is gone; the `?forget` handling moved up; a pack missing at start reloads into the site; the
  boot error's links gain "Settings & files" (`?site#settings`). Title / manifest name / favicon: OpenTennis, `logo.svg`.
- Page: Discord bar → logo band (logo left, "Tennis, the way it played on PS2." capped at 68px, Pick your ISO / Play +
  GitHub, darkened autumn-court shot) → Pick your ISO panel with the Needs box (SCUS-97610 v1.00, .iso, JP/EU won't
  work) + 3 steps → About (original / rewrite cards, platform tiles to the latest release, Online / Play anywhere) →
  Mods (+ opentennis-mods link, mod character select shot) → Settings & files → Donate (Ko-fi, "support my work",
  stats.json counts) → footer (nav, Discord, GitHub, not-affiliated line).
- Settings & files (same storage as the game): username (`./online.txt` `name`, the font's glyphs, 24 max), Hugging
  Face account (`hf-token.txt`: signed-in name + Sign out; sign in by pasting a token, checked with
  `huggingface.co/api/whoami-v2` like `modstore::sign_in`, read-only tokens refused; the token is sent nowhere else),
  `./settings.txt` editor (grouped selects/toggles/volume sliders for every key the game writes, unknown keys under
  Other, Reset), installed mods (OPFS `mods/<id>/mod.json`: name, kind, source, size; Remove / Remove all), storage
  (estimate + bar; disc data → Forget disc, character cards, offline copy (Cache Storage + store listing cache), desync
  reports → Clear, Keep my data (`storage.persist`), Erase everything).
- `web/logo.svg`: black backdrop removed, viewBox cropped to the art (195 108 1040 820) as v4 asked; `tools/apk.sh`'s
  sed is now a no-op and its `-trim` gives the same badge.
- `tools/web.sh` copies `docs/screenshots/*.jpg` into `web/shots/` (git-ignored) for the site's pictures.
- `web/touch.js`: while the site is up no pad, no fullscreen-on-tap, no edge-swipe guard, text selectable (the old
  `offsetParent` check failed for the now fixed-position page).
- `web.rs` unchanged: nothing needed from the wasm (the site must work without downloading it).

## Checked (headless Chromium over CDP, fresh profile, localhost)
- 1280×4200 and 390-wide layouts (one column on the phone), screenshots in the scratchpad only.
- Seeded storage: settings edit wrote `renderer = webgl2` in place; username saved with `#`/☃ filtered; mod Remove and
  sizes; Forget disc flips Play → Pick your ISO; Sign out empties `hf-token.txt`; a fake token got Hugging Face's 401
  through CORS (so the real sign-in path works from the page).
- Real ISO picked through the site: imported, game booted to its main menu ("started in 82 frames"); reload with the
  pack goes straight to the game; `?site` shows Play and its click drops `?site` and starts the game.

## Not verified / not 1:1
- The concept artifact (claude.ai) couldn't be opened: layout follows its written description, not the artifact pixels.
- Fonts: no external font is loaded (no third-party requests on the page); M PLUS Rounded 1c is used only if
  installed, else ui-rounded / system fallbacks.
- Real phones (iOS Safari, Android Chrome) not tried: scrolling inside the fixed page and touch.js's gating were only
  checked in desktop Chromium's mobile emulation.
- Settings edited while a game tab is open can be overwritten by that tab's next save (the page says changes apply
  at the next start).
- No in-game Extras › Support screen (dropped from the concept in v2; M97 territory).

## Also
- `update::tests::versions_in_semver_order` failed on the base commit: the alpha→beta rename (17d2c0d1) turned its
  list into `beta.1 … beta.10, beta`, which isn't semver order. Its first three entries are `alpha.N` again (test
  only; `newer()` untouched).
