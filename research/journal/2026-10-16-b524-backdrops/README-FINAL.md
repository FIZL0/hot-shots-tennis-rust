# B524 — sharp splash backdrops, baked in a match

The splash (B516) cycled the disc's court photos, which are soft at 1080p. Now a match draws its own court once into
a PNG the splash shows sharp. Disc photos stay as the fallback for courts not baked yet.

## What it does

- `crates/hst-play/src/play/backdrop.rs`: once per match, 0.6 s into a point's end (`Phase::Post`, the last point's
  at the match end too) on a clear or cloudy sky. If the court has no PNG that fits the window, an extra `Camera3d`
  (order −20, its own `Rgba8Unorm` image, `RenderLayers::layer(COURT_LAYER)`) draws for 3 frames. Then a
  `Readback` reads the image and an IoTaskPool task writes an RGB PNG. Nothing in the sim is read or written apart
  from `Game.phase`, so the RNG and timing aren't touched.
- Layer: `hst_base::COURT_LAYER` (9). The disc court load (`main.rs` `spawn`, clouds) puts it on through
  `Propagate(court_layers())`, and court mods put it on their meshes directly. Players, ball, effects, NPCs and HUD
  aren't on it.
- Size: the window's, capped at 1080 lines (2560 columns); a new bake when lines change by more than 20 % or the
  shape by more than 10 % (`fits`).
- Camera: one pose for every court, `(3, 1.4, −9)` Bevy space, yaw 2.72, pitch 0.2, vertical FOV 0.7. That is low on
  the −z half, off to the +x side, looking across the net to the other end with the umpire's chair at the net's right
  end, like the disc's court-00 photo. All 11 courts were checked at that pose (`context/b524/tunec1..11/`); each
  shows its landmark buildings or scenery.
- Storage: native `cache/backdrops/<key>.png` beside settings.txt (`store::{read,write}_backdrop`, written through
  `.part` + rename). Browser: card store `backdrops~<key>.png`, which `cards_keep` keeps. Keys are `court03` or
  `mod-<id>`.
- Splash (`main_menu/splash.rs`): the cycle includes disc courts plus court mods that have a picture or a bake.
  Only the PNG headers are read to list sizes. The current and next courts' bakes are decoded off-thread one at a
  time and dropped when leaving the press screen. A bake is drawn as one sharp tap dimmed to 60; others keep the
  blurred disc/mod picture.
- Env: `HST_BACKDROP=off` (never), `on` (as in a match, `--shot` too), `now` (`HST_BACKDROP_AT` s into any match,
  always overwrites); `HST_BACKDROP_VIEW=x,y,z,yaw,pitch[,fov]`. A plain `--shot` run doesn't bake.

## The pitfall

The first bakes had no ground, players or hedges: every opaque and alpha-tested `GsMaterial` was missing, while the
blended ones (nets, posts, trees) drew. The cause was `graphics::direct_draws`, which adds `NoIndirectDrawing` to each
3D camera one frame after it spawns. A view that switches to direct draws after its first frame never draws its
binned (opaque/mask) meshes again. Spawning the bake camera with `NoIndirectDrawing` fixes it (it stays fine at 3
frames). This applies to any future off-screen 3D camera.

## Checks

- `tools/check.sh -p hst-play backdrop` (3: sizes/refit, row unpadding, keys) and `-p hst splash` (3) pass.
- `HST_BACKDROP=on`, all-CPU match on court 2 (`--pads -,-,-,-`): baked at the first point's end, no players in it.
- Splash `HST_MENU=press` shots at 3 s and 14 s show the court-00 and court-01 bakes sharp, then the cycle goes on
  (`context/b524/splash/`).
- `cargo check --target wasm32-unknown-unknown -p hst` is clean.

## Not verified / not 1:1

- The pose matches the disc photo by eye on court 00 only (photo not at the same spot: our clock tower sits ~7 % to
  the right). The other courts' disc photos weren't compared, and they may be shot from other angles. The menu is
  ours to design (PLAN), so one pose was kept.
- The sky is the match's (deep blue on court 00) rather than the photo's pale, lens-flared sky. Clouds stand around
  the match camera's eye, so their spread differs from bake to bake.
- Player shadows on the ground can show in a bake (the shadow pass isn't layer-filtered). None were visible in the
  checked bakes.
- Court mods: the layer and key paths are written but not run (no court mod installed in the test folder).
- Browser (OPFS card store) and phones: compiled for wasm, not run. The extra frame's cost on a slow GPU isn't
  measured (3 frames, once per match).
- Rain/snow matches never bake, so a court only ever played in bad weather keeps the disc photo.
