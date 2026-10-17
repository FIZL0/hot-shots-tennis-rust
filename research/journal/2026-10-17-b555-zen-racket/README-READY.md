# B555 — Zen draws the cowboy's racket in pieces (blocked: not reproduced)

Report: context/reports/2026-10-17/2.png. It's from the same 4-browser online doubles match as B556's desync file
(`desync-1792209661-host.json`). Court index 3 (stage 4, blue hard court). Seats: host Anya (mod `oob_pc12_anya`,
no racket.glb, so the donor Miranda's disc racket), Lola c6, Big Chief c5 (the cowboy, 3P), Miranda c1. All four
were in browsers.

## What the picture shows
Zoomed in (`context/b555/rep_c.png`), the racket isn't a second copy. Its three parts each sit somewhere different:
- the gut (strings + red logo, the only blended part, drawn in the transparent phase) is in the right place, above
  the hand in the ready stance. It looks "faint" because nothing opaque is behind it;
- the grip (opaque) is in the hand but turned the wrong way, along the forearm;
- the frame (opaque) lies to his right, on its own, at a third orientation.

All three are children of the same `Racket` joint entity with `Transform::default()` (`character.rs` `spawn`), so they
share one GlobalTransform. Only per-mesh GPU data can split them. The opaque parts went through the binned phase
(batched, CPU batching on WebGL2: per-mesh `MeshUniform` in a dynamic-offset uniform window, first instance emulated by
wgpu's GL backend). Those two read some other slot's or an older frame's transform. The sorted transparent phase read
the right one.

## What I tried
- Native `HST_WEBGL2=1` (the B385 mode: WebGL2 limits, CPU batching) vs default: the report's lineup on stage 4
  (`--chars 8,10,7,8 --outfits 0,6,5,1`, with and without `--slot-mod 0` Anya), stage 1, Big Chief c0..9.
  The racket was right every time (`context/b555/run1..4`).
- Headless Zen (`tools/web_zen.mjs`, WebGL2 build, real AMD GPU through Gecko's GL: "Radeon R9 200 Series, or
  similar") on stage 1 and on stage 4 with the report's lineup without the mod: right in every 3–4 s shot
  (`context/b555/zen1/m.png`, `m2.png`).
- The ceremony (`HST_STATS=3`, Big Chief c5 ×2 winning): its free-racket motions are right too. It isn't the
  report's moment anyway: the ceremony camera faces the players and hides the markers, while the report shows the
  3P marker from behind (`context/b555/run6`).
- Reading the code: `extract_meshes_for_cpu_building` re-extracts every visible mesh each frame (no stale transform).
  `batch_and_prepare_binned_render_phase` (third_party/bevy_render) and `render_batchable_meshes` pair bins and
  batch sets by position, consistently. wgpu-hal GLES's `first_instance` emulation sets `naga_vs_first_instance`
  right before each draw. I found no CPU-side path that would hand an opaque racket part another slot.

## Leads for whoever picks it up
- What the report has that none of my runs had: an online match (rollback, snapshots, B556's desync and resync in that
  same match), a close camera behind 3P (cut-away or replay camera?), a windowed Zen (not headless), and four players
  on four browsers.
- With per-part orientations that look like the same racket at other times, the most likely cause is GPU-side stale
  uniform data: a `MeshUniform` window not written, or bound at a stale offset, in Gecko's WebGL. Next step: capture
  in the user's Zen when it happens. Does it persist across frames? Does a local match on that court and lineup in
  the same browser show it? Then, if reproducible, try `NoAutomaticBatching` on the racket parts, or a WebGL-only
  re-write of the uniform buffer every frame.

## Also changed
- `tools/web_zen.mjs`: the site text has shown from the start since M107, so visit 1 set the ISO before
  `site.js` had attached its picker handler, and the import never started (10 min wait, empty status). It now waits
  for `#iso`'s `onchange`.

## Not verified / not 1:1
- The bug itself: not reproduced natively (WebGL2 limits) or in headless Zen on this machine's GPU, so no fix was
  made. It needs the user's setup (windowed Zen, online 4-player match) or more detail from the user.
- Not tried in Chrome on the same frame, because nothing in my runs differed.
