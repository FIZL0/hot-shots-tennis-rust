# B272 — colour shift as Change Sides leaves: not reproduced (port or original)

## Port (native, the user's settings.txt: 100 % scale, high shadows, MSAA 4, uncapped)
- **Overlay camera on/off.** I forced `CallCamera` active all the time (a temp env switch) and took `--shot`s at 3 s,
  with and without it. Mean per-channel difference: 0.002–0.004 of 255 (abs 0.03, dither noise). Turning the camera
  on or off doesn't touch the scene's colours.
- **Every tick of a change of ends.** Bot doubles on stage 10, the ruins court, as in slot 5. I added a temp probe
  that saved one screenshot per tick through all 81 ticks and 40 frames after (`context/b272/run4/`,
  `context/b272/run3/`, sheet `run4/sheet.png`). The pop-up fades out over ticks 16→9. The phase ends at 0, when the
  camera turns off. Mean colours of regions away from the pop-up (top strip, bottom-left court, right side) drift
  smoothly by ≤ 1 unit over the last 20 ticks as the glide settles, with no step at the fade or the phase end.
  Consecutive frames have a median per-pixel difference of 0 on flat pixels. After the phase, the edge regions hold
  steady to ±0.05 while the score panels come up.
- Nothing global hangs on the pop-up. `effects::model` adds no light, ambient or exposure. `hud_gamma`'s
  composite/persist shaders are neutral (glare only blurs while the camera faces the sun, at the glide's start).

## Original (copy 3, slot 5)
- `research/b272_ends_shots.py 5 OUT 217 [k…]` saves scratch slot 8 before the change of ends. Then, per shot, it
  reloads that state and asks for a screenshot a fixed number of frames early, so the shot lands near counter k.
  vsync is game RAM, so each reload replays the same frames. (The hotkey is slow: about 217 frames until the call
  returns. It also varies, so the counters are approximate.)
- `context/b272/orig3/` (`sheet.png`): the pop-up is fully opaque in the shot nearest counter 0 and gone 30 frames
  later. Across those two shots, flat pixels have a median difference of 0 and mean differences of −0.04, −0.02 and
  −0.5. No global shift, as in the port.
- PCSX2 wrote shots of two sizes (674×379 and 616×347), so only same-size pairs can be compared pixel by pixel.

## Verdict
Neither the port (native Linux) nor the original shows a colour step as Change Sides leaves. So I have nothing to fix
without knowing where the user saw it.
**Needs the human:** say where it shows: the browser build (WebGPU or WebGL2) or native, which court, singles or
doubles, a human match with your own camera or bots, and the window size and settings. A screen recording of the
moment would show which frame changes.

## Not verified / not 1:1
- Browser builds (WebGPU/WebGL2), other courts, human matches with the player's own camera: not tried. I can't drive
  the browser build unattended here, and the report doesn't say which setup it was.
- Possible timing difference, unconfirmed: the original's pop-up still looked opaque around counter 0, while the
  port's fades out over ticks 16→9. The screenshot lag isn't constant, so the original's frame numbers are only
  approximate. A RAM read of the model's alpha (or a lock-step capture) would settle it. If the user's "shift" is
  the fade itself, this is the lead.
