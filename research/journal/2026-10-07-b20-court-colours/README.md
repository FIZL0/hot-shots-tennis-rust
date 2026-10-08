# B20 court colours — FINAL

The court looked dull because VU1's light was hard-wired (ambient 0.5, white light 0.49 from (1, 2, 1)/√6). In a
match the game lights the court with the scene light of the match camera:
- colour = the season's `envir_cNN.dat` light row RGB × row +0x10, ambient = the same RGB × row +0xc (the row
  `court_clear` reads; `gs::court_light`);
- direction = the sun (`shadow::Sun::light`, before the 50° clamp the shadows use);
- the specular half vector is taken between the light and the camera's forward axis, not a fixed vector.
- Not ported: the game dims both terms when the camera looks within ~37° of the sun. A match camera never does.

`research/tools/gsdump.py` lists a GS dump's draws (used to find the light registers).

## Check (stage 10 grass, doubles season 0, vs save slot 5)
Mean RGB of a grass patch between the net and the service line (`context/shots/b20/`):

| | R | G | B |
|---|---|---|---|
| original (orig_s5.png) | 90 | 98 | 46 |
| port before (port_s10.png) | 77 | 85 | 45 |
| port now (port_s10_light.png) | 94 | 101 | 53 |

The remaining difference is about +7 in blue (grass a little less saturated). Only court 10 was compared: slot 5 is
the only save the bot match can start from, and choosing a court from the menus isn't automated.
