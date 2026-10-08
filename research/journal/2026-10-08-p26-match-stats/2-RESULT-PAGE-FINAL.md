# P26b: the result page, the slide, and the stats-page details

## Result object (3a9f60 input/step, 3acf40 draw, 3ad2a0 page draw, 3aeba0 pulses)

Fields: +0x2364 page, +0x2368 slide x, +0x236c direction, +0x2370 target page, +0x2361 stats page seen, +0x237c timer, +0x23d4 blink, +0x2620-0x2634 pulses.

### Pulses
- Alpha with flag 0 is `0x66 - c*0x66/0x14`; with flag 1 it is `c*0x66/0x14`.
- Scale with flag 0 is `(0x14-c)*0.015+1`; with flag 1 it is `c*0.015+1`.
- c counts down from 0x14 and the flag flips below 0. The blink counter steps mod 40.

### Inputs
- X with timer >= 120 gives input 1 if the stats page is unseen, else Continue (sound 9/0/2).
- Otherwise Right (d-pad or stick) gives 1, Left gives 2, and X while unseen gives 1.
- Input 1 needs page != 1 and input 2 needs page != 0; each plays sound 9/0/0xc.
- The slide moves +-40 a tick; past |640| the page switches and is marked seen. The timer counts up after each step.

### Draw
- Bottom bar:
  - With timer >= 120: KeyAssign (0,48,152,24) "Continue" at (288,404); if blink > 0x13, info (144,160,32,32) at (256,400); then info (224,96) X at (256,400).
  - Always: KeyAssign row (page==0)*24 (Stats Screen / Score Screen) at (472,404), then info (192, page0 ? 64 : 32) at (440,400) when blink < 0x14, else (144,128).
- Pages: the current page is drawn at x = slide; the target page at slide -+ 640.

### Textures
- KeyAssign_inpane.tm2 (OTHER.XB0, rows 0 Score Screen / 24 Stats Screen / 48 Continue / 72 Cancel).
- info.tm2 from INPANE.
- Result background i_pause_result_15.tm2 (src 0,0,8,8 stretched over x0,80,w,256 at alpha 64).

### Page 0 (board mode 2, 3a7890)
- The set layout shifted by x = slide and y - 40.
- The games digits are always on Board(1), at full alpha unless the other team won more games that set.
- A highlight quad Board(0) (96,16,4,4) over the winner's stripe uses the pulse alpha, drawn after the stripes and before the frame.
- The winner's sets digit uses the pulse scale.

### Banner
- Model azuma/inpane/mdl/i_gameset_00 (38e450/38e870) with rotY pi and z 10.
- 3a9f60 sets scale 1.7 every frame while state < 2 (38e870's 4.5 is overwritten).
- Placed by 38ed40 at screen point (x+0x141, 53).

## Stats page (P26 left-outs)
- The "Stats" title fades top to bottom from (111,114,58) to grey 127. The port draws it as 40 one-pixel strips. The row names are plain white.
- The background behind the columns is i_pause_result_15 at half alpha.
- Under the faces sits the player's rank label (Lv) in the rank colour, not a character name. On the left half it sits at face+61, 4 px further left for even players; on the right half it sits at the label x; y is 104.

## Port
- crates/hst/src/play/match_stats.rs (Screen::step/page_x, bar, banner via effects::model on the call layer) and popups.rs final_board (Final {highlight, scale, dx}).
- `HST_STATS_PAGE=1` opens the stats page directly.
- Tests: match_stats slide, bar_items, rank_left, singles (background, rank, gradient). Capture script: research/p26b_result_shots.py.

## Verification
Checked against PCSX2 shots (context/p26b/s1_t125.png result page, page1b.png stats page) next to port shots context/p26b/port/p0.png and p1.png. The banner spans x 176-472 in the port against 176-467 in the original, the board sits at x0 88, and the labels and icons match.

## Gotcha
With the PCSX2 texture pack (replacements/ beside the ISO) the info.tm2 icons don't draw in PCSX2 shots. Point a scratch symlink of the ISO elsewhere to turn the pack off.

## Gaps (P26c)
- State 0, the winners' ceremony before the result page (~202 frames: NARROW WIN/DOMINATION/ANNIHILATION from result_gameset05, the team name, confetti, the winner camera). The port skips it and runs the banner animation on by 202 ticks.
- The left stick isn't read.
- The COM rank uses row 0.

## check.sh
All pass except hst-sim foot_extras_s05, whose fixture foot_s05x.bin exists only in the main checkout's context/fixtures (unrelated, from P17p).
