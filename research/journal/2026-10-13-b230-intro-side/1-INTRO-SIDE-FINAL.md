# B230 — the intro's camera starts on this machine's side; sounds follow the shown camera

## Cause

The intro holds the fixed loop (`intro::hold`), so no match tick runs till it ends. Online (B218) this machine's
own camera (`own_camera::OwnCam`) was only made by the first FixedUpdate tick, and the shared camera wasn't stepped
either: through the intro's match-camera part (after the flyover, the banner) every machine showed the shared
camera's unstepped resting view behind −z. The first tick then made the own camera and cut it to the +z end on the
guest: the snap.

## Fix (`play/own_camera.rs`, `audio.rs`; sim untouched)

- `link` also runs in PostStartup right after the match setup, and makes the own camera already turned for its
  player (`end < 0`) with a cut pending.
- `shown`: before the match's first tick (`g.instant.ticks == 0`) it shows the view the camera's first step cuts to
  (a copy stepped on the setup's scene: the own camera online, else the shared one), so the intro's match camera
  and the first tick's picture are the same. Offline this also removes the small jump from the unstepped resting
  view to the first fitted one.
- Positional audio: the original's listener is fixed behind −z (`hst_sim::sound::place`). `audio::set_turned`
  (set every frame by `own_camera::listener` from the shown match camera's end: the own camera online, else the
  shared one's `turned`) turns it round the court's centre: `play_at` places the mirrored spot (bearing and
  distance from the +z end), `play_toward`/`turn` (gallery, rain) add 180°. The intro already uses the right end
  (the own camera exists from setup); after a change of ends it follows the camera's `turned`.

## Verified

`../2026-10-13-b218-own-camera/pair.sh` (host + guest on this machine), shots in `context/b230/`:
- `before_s5` (fix reverted): at 5 s, during the banner, the guest shows the host's end; at 8 s (`fix_s8`) its own.
- `fix_s5`: during the banner host and guest each behind their own player (opposite ends), the same view the match
  then keeps (`fix_s8`). No desync / panic in the logs.
- `tools/check.sh -p hst`: 257 passed.

## Not verified / not 1:1

- Positional audio is not the original's in solo games: the original's listener never turns, even when its camera
  does (human on +z); now it turns with the camera (the task's request). Stereo checked by reading the code only,
  not by ear.
- Local 2P through the intro: `--shot` runs with `HST_INTRO=1` (via a wrapper) didn't show the intro, so only the
  match start was shot (`local2p_s6/12`, behind 1P at −z, no flip). Local multi-human games keep the setup's end
  (player 0's), which the preview uses too, so no flip is expected; not seen through the intro.
- Online from two separate machines: both peers ran on this machine (same code path; the camera only depends on
  seats).
- During the change-ends glide (the shared camera's) the listener follows the own camera's end, which can differ
  from the glide's for its few ticks.
