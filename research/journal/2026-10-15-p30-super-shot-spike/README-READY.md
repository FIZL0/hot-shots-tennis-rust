# P30 — iOS: super sweet spot lag spike

User (build 2012, court 2): a spike on a super sweet spot shot; that log had no frame over 40 ms mid-match.
Suspects: a first-draw pipeline compile of the hit effects, or the umpire call burst (P33).

## Finding
New `HST_FPS` diagnostic (`graphics.rs` `log_pipelines`, render world): every render pipeline as it's first
queued, with the app time, label, vertex attribute count, MSAA samples and shader defs. On iOS the fps log is on by
default, so the next hst-log.txt shows it too.

Desktop bot match (`HST_AUTOPLAY=1 HST_FPS=1 --play --court 2`, ~150 s, `context/p30/run1/log.txt`): the loading
screen came down at 2.8 s, then **two pipelines compiled mid-match**: `pbr_alpha_blend_mesh_pipeline`, 1 buffer
with 3 attributes (position, UV, colour), MSAA 4, at 6.81 s and 7.20 s (one plain, one BINDLESS) — the first
hit's dynamic effect meshes (sparks, ball-flight ribbon; the swing trails share the layout). Nothing else compiled
for the rest of the match.

Cause: the loading screen's warm-up (`mode.rs` `warm`, B110) shows every mesh under the cover so its pipelines
compile there, but B328's `effects::hide_empty` puts an effect mesh with nothing to draw on `RenderLayers::none()`.
Every hit effect is empty while loading, so no camera drew them and their pipelines compiled on the first hit of
the match. On desktop/Vulkan that's async and just skips the effect for a frame or two. On Metal, building a
pipeline means compiling the shader on the device. That can stall the frame, and the effect only appears once it's
ready. A first hit that's a super sweet spot shot (the big spark and glow) would make the spike easy to blame on
that shot.

## Change
`mode.rs` `warm`: a mesh on no render layer is put back on the default layer for the warm-up (`Warming` remembers
it). `unwarm` puts it back on no layer if it's still `effects::Emptied` (one filled meanwhile stays visible).
`effects::Emptied` is now `pub(crate)`.

Run 2 (same command, `context/p30/run2/log.txt`): 93 pipelines total, the same as run 1. All of them, the two
3-attribute blend pipelines included, are now queued before the loading screen comes down, and none after it in a
~150 s match. Test: `mode::tests::the_warm_up_draws_emptied_effects`.

Slow frames in both runs are this machine being shared with other agents' builds and PCSX2s (spread over the
match, not tied to hits), not the effects.

## Not verified / not 1:1
- On the iPhone: no device here. Whether the spike the user felt was this compile (most likely the match's first
  hit) or something else wasn't measured. The new `pipeline at` log lines will show it: plan/HUMAN.md has the check.
- A super sweet spot shot itself wasn't forced. Its effects use the same meshes and materials (only the texture
  handle changes: same pipeline), and no pipeline compiled in either 150 s bot match after the fix. Whether the bots
  hit a super shot in those matches wasn't checked.
- P33's umpire call burst is a separate suspect, handled there.
- Presentation only: no PCSX2 comparison needed (PLAN: effects only have to look right; sim untouched).
