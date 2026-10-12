# B74b — beach sky vs the original

## How the shots were taken
- No menu drive needed: PCSX2 state slot 10 is a beach (court 2) doubles game, slot 7 court 1 (P1 waits for a serve:
  ○ every 1.5 s from `tools/vpad.py` in the same `tools/pcsx2.sh` call), 3 court 4, 5 court 10.
- `research/b74b_hunt.py slot secs tag [lim] [max]` runs the game until the main view looks level or up (forward y
  at 0x1e7d34 < lim) and has held for a vsync. It then pauses, sends F8 and steps one frame (R1/B63's frame-exact
  recipe), and prints the view (eye 0x1e7d20, forward 0x1e7d30, down 0x1e7d40, tan 0x1e7d64 = `HST_EYE`) plus
  whether it stayed the same across the step.
- Port: `HST_EYE` set through a wrapper `HST_BIN`, because `tools/shot.sh` starts hst through `hyprctl exec`, which
  doesn't pass the shell's environment on (so B74's `port_c02_up.png` may not have been taken from the eye it
  meant). Disc textures came through an `HST_ISO` symlink. Shots are in `context/shots/b74b/`.

## Found: untextured draws were twice as bright
The hard-edged band is the clear colour showing above the open top of the sky cylinder: resort_sky's top ring is at
game y −1000, r 2484, which with lift 170 is 18.5° above the eye. Below the band the port's dome was much brighter
and more cyan (top ring (43,118,253), with blue clipped) than the original's (38,80,168). The original's top ring
meets its clear colour (37,77,165) with no edge.

Cause: the dome is **untextured** (vertex colours only; top ring (2,18,56), material ×1.99). VU1 sends
GS = min(⌊vc·mat·K⌋, 255) (P17s6, checked on a GS dump). An untextured prim draws that value as it is, so 0x80 is
128 of 255. `gs.wgsl` kept it in 0x80 = 1.0 units (right for MODULATE's texel × colour) and then wrote 1.0 as 255,
which doubles every untextured court draw. The fix is ×128/255 on the untextured branch. `court_clear` already used
the literal value: (3.98·0.902·1.4·198.9 + 152·56.1)/255 = 37, which matches the original exactly.

After the fix, the same view (slot 10, vsync 33316, held) matches the original down whole sky columns within ±2 levels
(e.g. x 1250: (38,80,168)/(37,79,168), (53,115,217)/(53,116,217), (62,137,245)/(63,138,247)). The band is gone.
The hills (bg), the clouds (clo), the umbrellas and the court lines coincide in `blend_c02_1_fix.png`.

## Other courts
- Court 1 (slot 7, two cut-aways): sky columns ±2 ((52,58,164)/(52,57,165)), and the building, trees and fence
  posts coincide.
- Court 4 (slot 3): a held level view (`c04_2`) and a moving one (`c04_1`): sky ±1 ((76,150,254)/(76,150,253)).
- Court 10 (slot 5): sky ±1 on `c10_1` ((23,58,126)/(23,58,125)).
- `tools/check.sh`: 323 tests pass.

## Not verified / not 1:1
- Most cut-away frames were taken while the camera was moving (the view read before and after the step differed by a
  few mm or mrad), so the eye given to the port can be a frame off. Only `c02_1` and `c04_2` held. Fine for sky
  colours and backdrop placement at this scale, not for pixel-exact overlays.
- Courts 3, 5–9 and 11: no save state there, so not screenshotted (needs a menu drive). The untextured fix applies to
  them by the same GS rule, but nothing checks it against a screenshot.
- Other untextured court materials (not skies) change by the same fix. Only skies were compared on screen.
- Gouraud colours: the port interpolates in float, the GS on 8-bit steps, so ±1–2 levels per pixel.
- Beach palm leans in the original, upright in the port → **B74c**.
- The court 10 cut-away `c10_0` shows no net in the original where the port draws one 0.38 m from the eye (a moving
  frame, unconfirmed) → **B74d**.
