# P23a — original 640×448 mode, fps cap, vsync

(2) The fps cap (30/60/120/144/uncapped) and vsync on/off were already in Settings → Graphics (P23, `graphics.rs`),
saved in `settings.txt` and passed to the match as flags. Nothing more to do there.

(1) **Original Mode**: a new row in Settings → Graphics (`original = on|off` in settings.txt, `--original` on the command
line or as a match flag).
- `main.rs`: `--original` also turns widescreen off (`WIDE`) and upscaled textures off (the disc's own textures).
- `hud_gamma.rs` `follow_window`: the scene image is 448 lines high, 480·aspect wide, so the centre 4:3 picture is 640×448
  with the PS2's non-square pixels. The HUD image is 448 lines with square pixels, so UI layout stays in the window's
  logical size. The composite samples the scene with a nearest sampler, and the HUD by `textureLoad` at `uv·size`,
  so both are scaled up unfiltered. The shader now uses `in.uv` for both images. That gives the same texels at
  normal size.
- `pixel_aspect`: Bevy builds the projection from the image's aspect, so each frame after `CameraUpdateSystems` this
  sets the scene cameras' `clip_from_view` to the window's aspect. The frustum keeps the image's aspect, which is
  slightly wider, so it only culls a little less.
- The setting applies live in the menu as well (the `Graphics` resource).

Checked with `tools/shot.sh` at 1280×720, comparing `--stage 1 --play` with and without `--original` (in
`context/p23a/`). With `--original`, the 4:3 picture has black bars and its court lines, net and players sit at exactly
the default view's centre positions, so the aspect fix is right. It uses the disc textures, and a zoomed crop shows
448-line nearest-neighbour pixels on both the court and the HUD. The Graphics screen fits all 7 rows.
`tools/check.sh -p hst`: 110 passed.

## Not verified / not 1:1
- Not compared side by side with a PCSX2 screenshot. Rendering only has to look right, and the geometry matches our
  default view, which was checked against the original earlier.
- The PS2's TV output isn't emulated: there is no interlacing, CRT blur or analog horizontal filtering, just plain
  nearest-neighbour scaling. That is the "no filtering beyond the original's output" reading of the task.
- The HUD draws at 448 lines with square pixels, not 640 columns. Non-square UI pixels would squash Bevy UI text,
  which is laid out in logical px.
- Scaling isn't integer: 720 / 448 = 1.6×, so rows come out uneven in height. There is no integer-scale option.
