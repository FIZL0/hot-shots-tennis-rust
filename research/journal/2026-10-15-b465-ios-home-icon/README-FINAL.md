# B465 — beta.opentennis.dev: no icon on iOS Add to Home Screen

## Findings
- The paths are fine on the beta deploy. `curl https://beta.opentennis.dev/icon-180.png` gives 200 image/png, byte-identical
  to `web/icon-180.png` (180×180 RGB, opaque). `icon-192.png`, `icon-512.png` and `manifest.webmanifest` give 200 too.
  The page's `<link rel="apple-touch-icon" href="icon-180.png">` is there. The site is at the domain root, so there's
  no base-path problem. pages.yml / tools/beta-site.yml copy the whole `web/` (web.tar.gz), so nothing is dropped.
- `/apple-touch-icon.png` and `/apple-touch-icon-precomposed.png` gave 404 on both sites. iOS tries those when it
  doesn't use the page's link.
- The manifest listed `icon.svg` (`sizes: any`) as a third icon. Newer iOS reads manifest icons, can't use SVG for a
  home-screen icon, and "any" can win the size pick. The apple-touch-icon link had no `sizes` either.
- `sw.js` didn't cache `icon-180.png`, so a page run by the service worker while offline had no touch icon.

## Change
- `web/index.html`: `sizes="180x180"` on the apple-touch-icon link.
- `web/manifest.webmanifest`: PNG icons only (180, 192, 512, `purpose: any`). The SVG stays as the page's favicon.
- `tools/web.sh`: copies `icon-180.png` to `apple-touch-icon.png` and `apple-touch-icon-precomposed.png` at the site
  root (git-ignored build output, like `shots/`).
- `web/sw.js`: caches `icon-180.png` and `icon-512.png` in the shell.
- Checked locally: `python3 -m http.server -d web` serves every icon path, the manifest and pkg/ with 200.

## Not verified / not 1:1
- Not tested on a real iPhone. No iOS device or simulator here, so I couldn't watch Add to Home Screen pick the new
  icon. Needs a check on beta.opentennis.dev once this deploys. If an earlier add cached a blank icon, remove the
  home-screen app first.
- Didn't run web_touch.mjs (Chrome installability) with a fresh wasm build. Its icon needs (192 + 512 PNG) are still in
  the manifest.
