# B405 — touch pad counted as a pad on Local

The on-screen pad is a gamepad (web: `navigator.getGamepads` gets `Touch controls (STANDARD GAMEPAD)` from
`web/touch.js`; iPhone/`HST_TOUCH`: `touch_pad.rs` connects `Touch controls (iPhone)`), so the main menu's `draw`
counted it among the pads: header "1 pad", Local tiles Touch + "Pad 1".

Fix: `draw` (main_menu.rs) counts the readable pads whose gilrs `Name` isn't a touch pad (`emote::is_touch`, now
`pub(super)`) and passes that to `drawn` as `controllers`, which only `home::layout` uses. `pads` (the entity list
for seat/controller numbering on the other screens) is unchanged, so the match's `--pads` indices still line up.

Check: unit test `touch_pad_is_no_controller`; desktop `HST_TOUCH=1` shot `context/b405/home-touch.png` shows
"0 pads" and only the Touch tile with 2P "Press ✕ / Enter to join".

## Not verified / not 1:1
- Not checked in a browser or on an iPhone (no device; the web touch pad is the same name check, `emote::is_touch`
  already matches its id). Front-end screen: no 1:1 requirement.
