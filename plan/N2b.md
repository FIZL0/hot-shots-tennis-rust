# N2b

- [x] **N2b — Ball size regression.** The ball is tiny now. It must be big, with the game's outline, like it
  was before. Suspect N2 (e06fa6a), which replaced the toon sphere with `ball1.mdl` + shadow. Check against
  the original (`tools/screenshot.sh` vs `--shot`).
  Fixed: the original draws `ball1.mdl` at scale 1 (ball object matrix, s03–s05); the app draws it ×2.4 (`BALL_DRAW_SCALE`, shadow too) with the black inverted hull, as the toon ball was.
