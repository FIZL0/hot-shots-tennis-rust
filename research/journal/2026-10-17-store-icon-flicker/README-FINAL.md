# B566 — mods store pictures flickered while loading

## The flicker the user saw (second pass, after their note: "when they're loading the first time")
The loading tiles' shimmer pulsed on `Browse::tick`, a count of rendered frames. The user's settings are
`vsync = off`, `frame_limit = uncapped`: the menu draws hundreds of frames a second, so the shimmer (0.12 rad a
frame, about 0.9 s a pulse at 60 fps) strobed on every tile still waiting for its first picture. `tick` is now 60 Hz
frames of real time (`Browse::clock`, advanced in `roll_time`), and the shimmer reads the clock itself; the
downloads' spinner, the search caret and a pack's before/after sweep, all written for 60 fps, keep their pace too.

## First pass (below): cached pictures stuck behind the fetches — a real but smaller issue

## Cause
`browse_pictures` read cached tile pictures only through `grid::want`, which stops once `cap` pictures are being
fetched (Downloads at Once, at least two; one while installs run). Any uncached tile earlier in the plan order took
the turns, so every cached picture after it waited — 8 rounds × cap a frame at best, all of them while a fetch was
out. Those tiles drew their loading face (a dim initial on the shimmer) for a frame or many, then popped to the
picture, row by row: the flicker. Network pictures had the same letter → picture swap.

## Fix
- `grid::from_cache`: before any fetch, every tile on view and the next screen without a picture is looked up in
  the cache (at most the tiles on view + 8 a frame); hits show that frame. Misses go in `Browse::uncached` so the
  disk isn't asked again each frame, and wait for `want`'s fetch turn as before (`got` takes them out again).
- A tile still loading draws only the shimmer (`Face::Blank`), no initial: an uncached picture replaces the
  shimmer, not a letter. The initial stays for a mod the store has no picture for.
- The offline sample screen (`HST_MENU=browsemods`) takes every picture on the first frame.
- Test: `mods_screen::tests::cached_pictures_skip_the_fetch_queue`.

## Not verified / not 1:1
- Not watched live against the real store: unattended run, and the flicker is a frame-to-frame effect a single
  `--shot` can't show. The cause and the fix are shown by the unit test (cached pictures read with both fetch turns
  taken). The front-end isn't from the original, so there's nothing to compare 1:1.

## Wrap-up note
`tools/done.sh` stopped on `hst-base mods::tests::rerigged_mod_plays_forehand_and_run` (rest·inverse bind off by
0.0037 on the local `context/mods/test_pc00` fixture). It fails on its own too, in a crate this task didn't touch;
the feet-ik merge (B563) changed the rerig code just before. Logged as a new B task, then B566 was ticked and
committed by hand. The `hst` mods_screen tests (31) pass.
