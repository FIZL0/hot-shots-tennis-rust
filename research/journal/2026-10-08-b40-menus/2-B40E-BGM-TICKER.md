# B40e — menu BGM and the description ticker

B40e was too big for one go: done here are the menu BGM and the description ticker, both measured on the original
(PCSX2 copy, PINE). The rest is split into B40e1–B40e5 (`plan/B40e.md`); the layout/sprite rects from the draw code
are B40i (1).

## Menu BGM (verified)

- The MENU overlay's track table (overlay data, 8+ entries): 0 `Menu/bgmm_03`, 1 `bgmm_04`, 2 `bgmm_05`,
  3 `bgmm_06`, 4–7 `bgmm_07..10`, 8+ `Court/bgmg_01..` (archives `SND/BGM/bgmm_NN.XB`).
- Its switch function runs only when the wanted index changes: stops the old track, loads the new one and starts it
  from its top at volume 55 (a constant beside the table), state flag 1 (steady). Its fade states (2 in, 3 out, ±1 a
  frame) are not used on a screen change. A second value (0x1e/0x23) goes to the SPU as an effect command (reverb
  depth?), not the volume.
- Measured live from the menu manager's index/volume bytes: Main Menu 1, Options 1, Game Selection 2, Character
  Select 2, Confirm Settings 3 (switches once the last player is picked); volume 55 everywhere.
- The original's Music option is "Turn music on and off during matches." — the menu BGM always plays, so the
  remaster's Music setting stays match-only.
- Remaster: `Screen::bgm` (Main/Settings/Controls → `bgmm_04`, Mode/Assign/Chars → `bgmm_05`, Confirm → `bgmm_06`),
  `bgm` system starts the track at 55 on a change; while the match (its own process) runs it is muted and restarts
  from the top on return. Test `bgm_per_screen`.

## Description ticker (verified on Options)

Glyph records (x, y f32, …, 0x28 bytes each) of the bar's text; per frame by lock-step and real-time VSYNC polling
(`context/b40e/`, slot 8 = Options):
- A new description enters from x 656 (GS 2384, offset 1728) at 32 px a frame: 20 frames to x 16.
- It holds at 16; the first move is 161 frames after the change (both measured on the autosave line).
- Then it moves left 2 px a frame; after its last glyph would pass x −8 it restarts at x 656 (no fast slide, no hold)
  and keeps going: the 81-glyph sound line last shows at −888, then 656, a run of 773 frames (1500-frame capture).
- Monospace, 11 px pitch; the music line (37 glyphs) never moves.
- Remaster: `ticker_x(frames, glyphs)` closed form, the draw system counts 60 Hz frames since the bar's text
  changed. Test `ticker_as_measured`. Shots: `context/b40e/r_settings.png`, `r_chars.png`.

## Not verified / not 1:1

- The bar's font: the original uses a small monospace font (≈9×14 px glyphs) not found yet; the remaster squeezes
  `word.tm2` glyphs (18 px high, ≤10 px wide) into the 11 px cells → B40e3.
- Which lengths scroll: 37 glyphs hold, the long lines scroll; the threshold (here: the text must fit 16..624) is a
  guess → B40e3.
- The bar's y and box (kept from B40, 404..436, text at 411) → B40i (1).
- The menu BGM's state while a match runs is not comparable (the original runs the match in the same process); the
  remaster mutes it and restarts the screen's track from its top on return.
- `bgmm_03` / `bgmm_07..10` (other modes' screens) are not used: those screens aren't in the remaster.
- The hand swing, slide-in/out, the select's and confirm's extras, random court/umpire: not done → B40e1–B40e5.
