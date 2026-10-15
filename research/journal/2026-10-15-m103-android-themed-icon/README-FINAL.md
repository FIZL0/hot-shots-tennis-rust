# M103 — Android themed (monochrome) launcher icon

`tools/apk.sh` adds a `<monochrome>` layer to `mipmap-anydpi-v26/ic_launcher{,_round}.xml`
(`@mipmap/ic_launcher_monochrome`, per density, same 108dp canvas / 64dp-wide badge as M100's foreground).

The launcher only uses the layer's alpha, so a plain alpha silhouette of the badge would be a featureless blob (the
badge has a solid outer ring). Instead alpha = badge alpha × badge brightness, levelled 12–42 % → 0–1: the black
outline (#0b0c0d) between ring and art drops out, so the lettering "OPEN TENNIS", the court lines and the ball stand
out of the ring. 12 % was picked over 20 % by eye (20 % thins the ring and the court lines fade).

## Verified
Own emulator (AVD hst34, API 34, `-read-only -port 5660`, x86_64 build): themed icons switched on through the
launcher's `content://com.android.launcher3.grid_control/icon_themed` (`boolean_value=1`), app dragged onto the home
screen (Android 14's launcher themes only the workspace, not the drawer): the badge shows tinted in the system's
two-tone palette; with themed icons off it's the full-colour badge on green again. `aapt2 dump xmltree` shows
background/foreground/monochrome in the adaptive icon. Screens: `context/m103/`.

## Not verified / not 1:1
- Only on the AOSP emulator launcher (Launcher3/Quickstep); not on a real phone or Pixel Launcher: no device attached.
- Android 13 (API 33) not tried separately; only API 34 AVD available. The `<monochrome>` element is the same API.
- 1:1 to the original does not apply (branding is ours).
