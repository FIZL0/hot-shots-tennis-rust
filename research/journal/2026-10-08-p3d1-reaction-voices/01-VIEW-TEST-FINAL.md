# The view test and the voices

## The routine (0x354050)

Runs per player while +0x3fa7 == 1 (reacting). +0x3ba0 counts frames up to 5; on reaching 5 with more than one
player:

1. 0x369530(1.0, 1000.0, _, model origin = *(p+0x1808)+0x30): the origin through the view matrix 0x1e7e30 must have
   0.001 < z and F/z < 1000 (F = 0x1e7ff0[0][0]). Fail: nothing, +0x3d38 untouched.
2. +0x3d38 = -1. Spot = +0x17e0 if +0x12bc == 0xd else +0x3d70.
3. cos of the level bearing eye (0x1e7d20) → spot against the level look (0x1e7d30, the camera's forward row) must
   be ≥ cos 50°, and F / z(spot) > 125 (80 when cos ≥ cos 5°).
4. Voice by reaction motion (+0x3db0) — table in `sound::reaction_voice`; a non-cheer/groan motion of the losing
   side ((+0x12b8 & 1) != 0x4230a8, the winning side) voices program 8, in singles only after a shared-draw
   %100 < 30 roll. gm+0x35c (instant replay) changes singles keys. Three players: nothing.
5. 0x3553d0 (the shouts' key picker): keys lo..hi minus the slot's last (+0x3b64 + 4·slot), one shared draw
   r15 % n, plays on bank slot 1 + player; the handle goes to +0x3d38.

F: 0x1e7ff0[0][0] · tan(0x1e7d50/2 in degrees) = 240.00 on every frame checked, so the app uses 240 / tan(View.fov).

## Recorder

`research/p3d1_view.py <out> <frames>` loads slot 5, steps lock-step and writes each frame's vsync, players,
winning side, replay flag, eye, look, view and projection matrices, fov, and per player +0x3ba0, +0x3fa4,
+0x3db0, +0x12b8, +0x17e0, +0x3d70, the origin, +0x3d38 and +0x3b64 (layout in `tests/rng.rs` `post_views`).
`tools/pcsx2.sh python3 research/p3d1_view.py context/p3d1/view_s05.bin 3650`: vsync 7572–11221, contiguous.

## Three point ends

| 5th frame | motions p0–p3 | winners | in view | voice |
|---|---|---|---|---|
| 8540 | 30 2c 34 33 | 1 | p1 (cos 1, F/z 152) | p1 program 7, key 1 (doubles 0..1) |
| 9816 | 34 33 32 30 | 1 | p3 (cos 1, F/z 207) | none: a winner's team reaction; +0x3d38 stays -1 |
| 10658 | 32 31 34 2d | 0 | — | none |

The rest are behind the camera or fail the bearing/size. p1's stale handle 9 survives 9816 (failed step 1).

## Key-on lag

The draw is in frame 8539 but spu_s05 keys the voice at 8551: player 1's bank's program 7 key 1 sequence has its
only key-on at frame 11 (`Bank::key_ons(7, 1)[0].frame`). The app's sequence player already does this.
hits_s04's reaction voices (8088 7/0, 8613 8/1, 9152 9/0) have no view recording; left unchecked.

## Port

- `sound::reaction_view`, `reaction_voice`, `Voice::key` (shared by the shouts) and `Voice::react`; `Voice` keeps
  six key-memory slots (shouts 0–1, reaction voices 3–5).
- `play/reaction_voices.rs`: counts frames since the reactions started; on the 5th tests each player against
  `g.cam.view` and voices. Ponytails: the player position stands for both origin and spot; a standing body-hit
  player has no motion (any non-cheer/groan voices alike).

## Tests

- `rng.rs` `shared_draws_like_the_game`: the 8539 draw now comes from the port (view_s05 sample 8540), key 1.
- `rng.rs` `reaction_views_like_the_game`: at every 5th frame, out of view keeps +0x3d38, in view clears it or
  sets it exactly when the port voices (2 in view, 1 voiced).
- `sound.rs` `shouts_match_the_game`: spu_s05's only programs 7–10 key-on is player 1's 7/1, sequence started 8540.
- `play/reaction_voices.rs` `matrix_is_local`: the app's view matrix agrees with `View::local`.
