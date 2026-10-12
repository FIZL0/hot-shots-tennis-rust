# B43 — loading screen with hints

The original's pre-match "Now Loading..." screen, drawn on `mode.rs`'s loading cover for the menu→match switch
(`crates/hst/src/loading.rs`, hooked via `Mode::cover`, set in `main.rs match_mode`). Up only while the port loads.

## Disc assets
- `MENU/MENU00B.XB0` → `Loading.tm2` (256×256): logo panel 9-slice, white pane + rim 9-slices, "Now Loading..." sprite.
- `MENU/WALL/WALL{h+1:02}.XB0` → `L_txt{h:02}.tm2` (512×256): hint h = 0..37, text and pictures baked in.
- `MENU/WALL/SUB{c:02}.XB` → `L_bg{c-1:02}.bmp`: 640×448 8-bit BMP, one per court c = 1..11 (= the port's `--court`).
  Checked: court 3 = Western on both the cover and the match; Sakuragaoka matched the real screen.

## Layout (640×448 screen)
- bg full screen; pane fill: uv cuts u (184,200,216,232) v (0,16,32,48), centre (320,223), 524×302; rim: same u,
  v (48,64,80,96), 516×294; logo panel: u (0,144,160,184) v (0,104,120,152), centre (312,200), 560×368.
- hint 512×256 at (64,96); "Now Loading..." src (0,152) 256×104 at (416,392).
- Draw order: bg, fill, rim, logo panel, hint, words. Sheet alpha via `decode_alpha8`; the fill comes out ~0.4 as in PCSX2.

## Hint picker (front-end menu code)
Per hint a seen byte; bit 4 = Fun Time Tennis, 2 = Challenge/Training, 1 = any. Fun Time lists:
first [1,2,3,7] (in order), second [4,5,6,11,12,13,15,28,29,30,31,35,37] (random). Welcome (0) first if its mode bit is
clear; then first unseen of the first list; then a random unseen of the second; all seen → random of both
(`(rand>>16 & 0x7fff) % n`). Marks `seen[h] |= bit|1` (welcome: `|= 7`). Real game check: after the welcome, Fun
Time showed L_txt01. Test: `loading::tests::hint_order`.

## Check
PCSX2 copy 3 (slot 2 → doubles → start) screenshots in `context/b43/run4/`; port shots `context/b43/port_*.png`
(4:3-ish, 3:1 ultra-wide) by eye: same art, positions, overlap and translucency.

## Not verified / not 1:1
- Challenge/Training hint lists and the challenge-state overrides (hints 28/29/32/33) not ported: the port has no
  such modes; Fun Time's lists only.
- Seen flags live in `hints.txt` beside the disc, not the memory-card save; random numbers are not the game's rand().
- Pane alpha matched by eye (~0.4), not measured per pixel.
- Wider than 4:3 the court picture fills the width, cropped top/bottom; taller it's letterboxed (original is 4:3 only).
- Shown only for menu→match; match→menu keeps the plain black cover. Load time not matched (by design).
- No fade in/out of the screen as the original might do; not checked.
