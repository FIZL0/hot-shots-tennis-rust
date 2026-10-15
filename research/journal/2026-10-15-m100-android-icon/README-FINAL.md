# M100 — Android launcher icon

`tools/apk.sh` now draws the launcher icon from `web/logo.svg` at build time (needs `rsvg-convert` and `magick`):

- the SVG's full-canvas black backdrop rect is dropped (sed), rendered at 2800 px wide and trimmed to the badge;
- `mipmap-anydpi-v26/ic_launcher{,_round}.xml`: adaptive icon, background `@color/ic_launcher_background`
  (#1d5c3a, the web icon's court green), foreground = 108dp canvas with the badge 64dp wide; its drawn corners
  (the "O" and the ball) stay inside the 72dp mask circle;
- legacy `mipmap-{m,h,xh,xxh,xxxh}dpi/ic_launcher.png` (48dp rounded square) and `ic_launcher_round.png` (circle);
- `aapt2 compile --dir` → `target/apk/res.zip`, linked with the manifest; `android:icon`, `android:roundIcon`,
  `android:label="OpenTennis"` on `<application>`.

`aapt2 dump badging`: label `OpenTennis`, icon `res/mipmap-anydpi-v26/ic_launcher.xml`.

## Verified
Emulator (AVD hst34, own `-read-only` instance on port 5640, x86_64 build): app drawer shows the badge on green in
the launcher's squircle mask, labelled "OpenTen…" (truncated by the launcher); recents shows the badge as the task
icon. A circle-mask composite of the xxxhdpi foreground shows nothing clipped.

## Incident
My first `emulator` launch failed (another agent's hst34 was running, non-read-only launch refused) and my
`adb install -r` went to that agent's emulator (emulator-5584), replacing whatever hst build it had installed with
this branch's build (app data kept). That agent's next install restores theirs. Use `-read-only -port N` and
`adb -s emulator-N` when emulators are shared.

## Not verified / not 1:1
- Not checked on a real phone or with a circle-mask launcher (only the emulator's squircle launcher + an offline
  circle composite): no device attached.
- Themed (monochrome) icons for Android 13+ not provided: not asked for; would need a single-colour silhouette.
- 1:1 to the original does not apply (front-end/branding is ours).
