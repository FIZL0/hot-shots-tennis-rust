# P32 — Grand Indoor Palace GPU cost

Task: the palace (stage/court 7) ran at 40 fps on the iPhone, GPU-bound, at the Phone preset (67 %, Low shadows, 2× MSAA).

## Stand-in
No iPhone here. The integrated GPU stands in for it:

    export HYPRLAND_INSTANCE_SIGNATURE=$(ls /run/user/1000/hypr/ | head -1) WAYLAND_DISPLAY=wayland-1
    HST_ENV="HST_PASSES=1" HST_GPU=igpu HST_WINDOW="1920 1080" tools/bench.sh LOG 30 --stage 7 --court 7 \
        --scale 100 --shadows low --msaa 2x --fps 0

It shows the same thing: the palace is ~18 % slower than court 2. The fps number is noisy because other agents share the
machine. The per-pass GPU ms are more reliable.

New tools:
- `HST_PASSES=1` (graphics.rs): every 5 s, logs the ten slowest render passes by `elapsed_gpu` (Vulkan has timestamps,
  Metal doesn't). A pass that stopped running keeps its last average: the `msaa_writeback` ~1 ms seen in a match is
  left over from the menus, because no match camera has the writeback pipeline.
- `HST_ENV` in `tools/bench.sh` passes extra environment variables through to the game.

## Where the time goes
Baseline, iGPU: court 2 opaque 2.0–2.6 / transparent 0.9–1.1 ms. Palace opaque ~3.1–3.3 / transparent ~3.2–3.7 ms.

The cost is the palace's hole model (`extra_h01_…`). Indoors it fills the screen, and each hole material is drawn twice:
once as the A ≥ 0x80 opaque pass (colour + Z, with discard) and once as the A < 0x80 blended pass (gs.rs
`for_court_batch`).
- Most materials (walls, stands) are solid: texture alpha 0xff and vertex alpha ≥ 0x80. Their blended draw discards
  every pixel, but it still rasterises and shades the whole screen.
- The floor materials (512² textures) have vertex alpha 0x6c everywhere. That means the floor is drawn *only* by the
  blended pass (real work, ~1.5 ms), while its opaque draw discards everywhere.
- Cutout materials (see-through texels) need both passes (~1.1 ms of real blended work).

## Fix
`gs::hole_triangles` works out, for each triangle, whether a pass can draw any pixel at all. It uses the vertex alpha ×
material alpha and the texel alpha inside the triangle's UV footprint (`gs::SeeThrough`: summed-area tables per mip
level with a one-texel bilinear margin, honouring repeat/clamp). Then in `gs_models_anim` (main.rs), for hole materials
whose alpha is fixed (no `.MTA`, no `.UVA`, no live texture reload):
- the A < 0x80 draw keeps only the triangles that can pass;
- the A ≥ 0x80 draw becomes `Test::Always` (no discard, so tile GPUs can use hidden-surface removal) on triangles
  where A < 0x80 can't pass. It stays as the A ≥ 0x80 test only on the doubtful ones, and is dropped where it can't
  pass;
- empty draws are dropped, and partial ones become extra mesh parts.

Separately, the A < 0x80 shader now also discards alpha exactly 0. Such a pixel blends to an unchanged target, so this
is exact.

Result on the iGPU, A/B: opaque 3.20/3.12 → 2.15/2.15 ms, transparent 3.72/3.59 → 2.93/3.20 ms, fps ~82–84 → ~96–100.

Pixel identity: compared `tools/shot.sh … --shot-at 2 --msaa 2x` with a fixed weather seed, old path vs new. Court 7
overview and radius 12, plus courts 2 and 10 at radius 12: 0 differing pixels (screens in `context/p32/`). `--play`
shots aren't deterministic, so they couldn't be used.

## Levers not taken
- Rgba8 main texture (`CompositingSpace::Srgb` instead of the `Hdr` Rgba16Float). It would halve colour bandwidth,
  which matters most on tile GPUs with MSAA. But it needs clear-colour and blit/upscale `source_space` handling, which
  was too risky to do blind.
- Lower scale or MSAA at the Phone preset. Not needed on the stand-in, and can't be judged without the device.

## Not verified / not 1:1
- Not run on an iPhone (no device). The 60 fps target at the Phone preset is unconfirmed. To confirm: run the IPA on
  the palace with `HST_FPS` (P28 gpu ms) at Phone 67 %/2×. Desktop iGPU stand-in only.
- Phone preset left at 67 % / Low / 2× without testing it on the device.
- Splitting an opaque draw into Always + A ≥ 0x80 parts could change the draw order between coplanar triangles at
  equal depth (z ties). Not seen: the diffs above are 0 pixels.
