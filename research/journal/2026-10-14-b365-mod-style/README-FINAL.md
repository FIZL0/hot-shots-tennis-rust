# B365 — mod style and complexity check (investigation only)

The user asked for investigation only: no code changes are committed. This entry and the task tick are the
only things in git. The prototype lives in `context/b365/` (git-ignored). The proposals below are for the user
to pick from; no TODO lines were added.

## What the mods weigh (costume c00, base-colour images; `context/b365/` scripts, numbers from `../open-tennis-mods/out`)

| set | triangles | vertices | prims/materials | joints | texture Mpx | largest side |
|---|---|---|---|---|---|---|
| HST disc (14) | 2645–2688 | ~4000 | 9–22 | 61–79 | 0.27–0.36 | 256–512 |
| Fore! (24) | 2488–3000 | ~4400 | 6–33 | 71–118 | 0.14–0.52 | 256–512 |
| Get a Grip / Open Tee 1-2 (46) | 576–747 | ~1400 | 3–9 | 67–68 | 0.09–0.12 | 256 |
| **Out of Bounds (17)** | **15.5k–24.2k** | **10k–17.5k** | 13–64 | 72–128 | **2.5–5.5** | **512–1024** |

- OOB is ~8× HST's triangles, ~4× its vertices and ~10× its texture area. Every other set is already within HST's
  budget (Fore! matches it: same engine generation).
- OOB costumes also embed **13 normal maps each (~2 Mpx)** that nothing draws. `mods.rs::materials_of` decodes
  and uploads every `doc.images()` entry, used or not, so they cost memory and load time for nothing.
- OOB `textures/upscaled/` is 425–553 MB per mod (39 PNGs per costume, up to 4096², **~59 Mpx per costume**),
  including upscaled normal maps. With "upscaled textures" on, that is roughly 236 MB RGBA per player before
  `materials_of`'s second (`raw`) copy. *Estimate from file sizes, not measured at runtime (see below).*
- Mod textures get **no mipmaps** (`Image::from_buffer` creates one level; disc textures carry their own). A
  1024² texture minified to ~30 px on screen at match distance aliases badly. That causes the "noisy/grainy" OOB
  look in play, and the sampling is cache-hostile too.

## Speed (indicative: `tools/bench.sh`, 4-bot doubles, 1280×800, other agents loading the machine)
- dGPU (RX 9070 XT), 4× OOB vs stock: ~120–150 fps vs ~180–225 fps; CPU 11–12 vs 9.5 ms/frame. VRAM 1189 vs
  931 MB (+258 MB for four OOB players at disc-res textures). The first frame (load) takes 1.37 s vs 1.0 s.
- iGPU (Raphael): both stuck at ~55–63 fps, CPU-bound, no measurable gap.
- One run with the same arguments fell to 60–70 fps from contention, so these are trends, not figures. The
  `--no-upscale` run showed the same VRAM as the upscaled run, so the bench evidently didn't load the upscaled
  set. Its cost is still unmeasured.
- Weak targets (phone/browser/Deck), where the user sees "much slower", weren't benchmarked. Memory there is the
  likely limit: the earlier phone crash fix (store preview pictures) points the same way.

## Can B364's Mesh setting fix it at runtime?
No, not on its own. A temporary local test (`lod::tests::disc_and_mod_characters_reduce` pointed at OOB, since
reverted) gave these numbers:

| model | 100 % | 50 % | 25 % | 12 % |
|---|---|---|---|---|
| OOB Jasmine | 23569 | 14293 | 10137 | 9217 |
| OOB Fernando | 24201 | 15029 | 11665 | 10787 |

It stalls at ~9–11k (3.5–4× HST) in 56–88 ms per character. Relaxing `MOVE` 5× (→8.5k) or `SKIN_APART` to
any skin (→7.9k) barely helped. The cap comes from the topology rules: collapses only onto existing vertices,
seams and borders kept, no texture flips. Only 13 % of OOB vertices are on borders and 4–6 % are morph-locked,
so those aren't the cause. It also only rewrites indices, so vertex buffers, skinning and morph-target cost stay
at 15k vertices.

## Style
Side by side (`context/b365/run1/crop.png`: HST pc06, Fore! Phoebe, OOB Jasmine, OOB Felipe, viewer pose):
- HST/Fore!: flat, bright, saturated painted colour; shading comes from the GS lighting.
- OOB: darker (mean HSV value 0.53 vs HST 0.71, Fore! 0.59; saturation ~equal 0.32 vs 0.34), photographic
  fabric grain and stitching (denim, knit, leather). The textures were authored for PS3 lighting with normal
  maps and specular, which we don't draw. At HST-sized footprints their fine detail is about the same as HST's
  (5.4 vs 5.5 high-pass energy at 128²). The grain on screen comes from texel density without mipmaps, not from
  the art itself.
- Proportions are already handled by the rerig (`size_why`: head scaled toward HST's ratio). Felipe still reads
  larger than HST players in the match shot (`match_pair.png`), worth a look in open-tennis-mods.

### Prototype (`context/b365/proto/hst_style.py`, not committed)
It rewrites a costume `.glb`: base-colour images box-downscaled to ≤256 px and brightened (HSV value^0.7);
unused normal maps replaced with 1×1 PNGs (emptying them crashes the loader, see above). On Jasmine and Felipe,
all 6 costumes go from 5.6 MB to 2.3–2.6 MB per `.glb`, texture area ~3.7 → ~0.4 Mpx (HST's budget).
The result is `context/b365/run1/proto_cmp.png` (pairs: OOB as is → prototype). The grain is gone and the colours
are brighter and flatter, much closer to HST/Fore!. Downside: the plain gamma also lifts true blacks (Felipe's
black shirt turns charcoal grey). A final version should lift mid-tones and keep blacks, for example a levels
curve, or match each texture's value histogram to HST's.

## Proposals for the user (none started)
1. **open-tennis-mods, `hst_style` pass for OOB** (and any future HD source) in `tools/rerig` packaging:
   - textures to HST's budget (≤256, 512 for a face/body atlas; ≤~0.5 Mpx per costume), mip-friendly box filter;
   - drop normal maps from the `.glb` and from `textures/upscaled/` (≈ 13 of 39 files);
   - a tone curve toward HST's brightness that keeps blacks;
   - offline decimation to ~3–4k triangles with an attribute-aware simplifier that also shrinks vertex buffers
     (e.g. meshoptimizer `simplifyWithAttributes`, with locked borders/morph vertices like `lod.rs`);
   - cap `textures/upscaled/` at 1024 (4096 on a 30-px figure is wasted).
2. **Remaster, complexity limit in the mod standard** (§7 conformance, `mods.rs::conform`/`check.py`):
   suggested numbers are ≤8k triangles, ≤12k vertices, ≤1 Mpx of base-colour texture, longest side ≤1024
   (2048 upscaled). Warn in the store/editor above them. Reject, or auto-reduce at load, above ~2× that. HST's own
   characters are 2.7k / 0.36 Mpx, so 8k is still 3× the original.
3. **Remaster, cheap load-side fixes** in `mods.rs::materials_of` (no format change):
   - decode only images a material uses as base colour (skip normal maps);
   - generate mipmaps for mod textures (look and speed);
   - downscale any image over the limit at load;
   - check whether both the `StandardMaterial` image and the `raw` GS copy need to stay resident.
4. Optionally a per-mod triangle budget for B364 (target count instead of a percentage). It would still need (1)
   or a relaxed reducer to get below ~9k on OOB.

## Not verified / not 1:1
- Mods are remaster-only additions; nothing in the original to match. Nothing in the sim was touched.
- Speed numbers are indicative only: a shared machine with other agents running. Upscaled-texture memory is an
  estimate from file sizes (the bench didn't load them). Phone, browser and Deck weren't measured.
- The prototype was checked in the character viewer on 2 of 17 OOB mods, costume c00 only, not in motion or in a
  full match. Its tone curve greys blacks (see above).
- Style comparison by eye and by simple texture statistics only.
