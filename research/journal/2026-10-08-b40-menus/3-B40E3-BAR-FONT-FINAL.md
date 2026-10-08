# B40e3 — the description bar's font and which lengths scroll

Both from the code, checked on the original (PCSX2 copy 3, slot 1 = Game Selection, RAM dumps over PINE).

## Font (verified)

- The bar is drawn by the boot program's text printer, not the MENU overlay's `word.tm2` font: the overlay's print
  routine walks the string (`07 xx` colour escape, `0b xx` icon escape, `\n` new line) and hands each glyph to the
  printer, which queues records (x, y f32, z, x/y scale, string, palette byte, RGBA tint, 0x28 bytes) and later draws
  one sprite per glyph.
- Sheet: `CMN/GFFONT.XB` → `data/common/font/ascii.bmp`, 256×256 4-bit. The printer's texture buffer in RAM
  (0x8000 bytes, PSMT4) is identical, pixel for pixel, to the BMP's picture (top-down, as an image viewer shows it).
- Glyph: cell `g = char − 0x1f` (unsigned, clamped to 0..224), 21 a row in 12×23 px cells; source 11×22 texels from
  the cell's corner (+½ texel), drawn 11 wide × 11 field lines (22 frame px), 11 px pitch; not drawn once its right
  edge is ≤ 0 or its left ≥ 640.
- Colour: the message's `07 7f` escape picks palette 0xd and tint 15 (0x8f8f8f, a 0x80). Palette 0xd (read live):
  RGB 200 for every index, alpha by index `0,0,0,0,0,1a,1a,1a,1a,33,33,33,4c,4c,66,80` (index 15 opaque). Ink =
  200·0x8f/128 = 223: the brightest bar pixel in a PCSX2 screenshot is 223,223,223, the remaster's too.
- y: the records' GS y 2140 = field line 204 = frame y 408.
- Remaster: `bar_font` bakes the sheet into RGBA (223 grey, alpha from the ramp); `Draw::info` draws cell g at 11×22.
  Shots `context/b40e3/r_{settings,mode,chars}.png`.

## Which lengths scroll (verified, from the overlay's bar update)

`n` = glyphs in the text (escape pairs and `\n` not counted), `t` = the bar's frame counter since the text changed:
- `n·11 ≤ 616` → x 16 (so up to 56 glyphs hold), else `x = 656 − 2·((t + 160) mod ((n·11 + 656) >> 1))`;
- while `t < 160`: `x = max(16, 656 − 32·t)` (the 16 is a constant in overlay data, read live).
That is exactly B40e's measured curve (in at 32 px/frame, held to frame 160, 773-frame loop for 81 glyphs), now with
the real threshold. `ticker_x` is this formula; test `ticker_as_measured` adds 56 holds / 57 scrolls.

## Not verified / not 1:1

- The GS blend and texture filtering for the glyphs weren't read (ALPHA/TEX1 registers): drawn with the remaster's
  normal alpha blend and Bevy UI's sampling, without the ½-texel offset. Ink colour matches; edges not compared
  pixel by pixel.
- The counter `t` is the remaster's 60 Hz frame count from wall time since the text changed (B40e); the original's
  counter wasn't traced to its writer, only its use.
- The bar's box and its y (404..436) stay B40's → B40i (1).
- The remaster's own descriptions (Main/Settings/Controls) and any character outside Latin-1 use the same sheet;
  a character past the last cell shows the last cell, as the printer's clamp does.
