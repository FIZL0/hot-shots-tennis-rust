# B459 — website platform tiles

- `web/index.html` (#disc tiles): each tile has an inline SVG platform icon (globe for Browser, Windows, Tux,
  Android, Apple), drawn in `currentColor`, plus a glyph on the right: download arrow (one shared `<symbol>`) on
  the four download tiles, play triangle on Browser. Download tiles now link straight to their asset:
  `releases/latest/download/opentennis-windows-x86_64.zip`, `…-linux-x86_64.tar.gz`, `…-android.apk`,
  `…-ios.ipa` (names from `tools/release.sh`).
- `web/site.css`: the tile is a grid `30px icon | name over note | 20px glyph`, fixed pixel sizes so every tile is
  laid out the same; minimum tile width 150 → 190px so the text column keeps room. Icons follow the tile's ink,
  so they work on the white and gold (Browser) tiles; the site has no dark theme, and if one is added the icons
  follow `color` automatically.
- Checked with headless Chromium at 1200 / 700 / 360 px wide (5 in a row, 3+2, one column): same layout everywhere.

## Not verified / not 1:1
- Download links not clicked against a live release (would publish nothing, but needs a release that has those
  assets; names match `tools/release.sh`).
- Only rendered in Chromium, not Safari/Firefox.
