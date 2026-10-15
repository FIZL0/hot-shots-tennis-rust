# B86 — select previews from a PNG cache, crash hunt

## Crashes found (scripted full-speed scrolling, `tools/select_stress.sh`)

1. **`Sway` inserted on a despawned rig** (B84's): the select despawned a preview's rig in the same frame that
   `noise::attach` queued `Sway` for it, so the deferred insert panicked. Fixed at both ends: `attach` uses `try_insert`,
   and the select despawns old preview entities in `Last` (the `Gone` list), after every Update system that queued
   commands for them has run. `shade::own_materials` (also on `Added<Rig>`) is covered by the same deferral.
2. **VRAM exhaustion → `DeviceLost` → wgpu teardown panic** ("Trying to destroy a SwapchainAcquireSemaphore that is
   still in use", exit 101; s2 found the same in B94). Every hovered character stayed loaded (an unbounded map) and the
   neighbours were prefetched as well. Each disc character carries its upscaled replacement textures, so own VRAM grew
   about 700 MB/s while scrolling until the device was lost. Fixed by not loading while scrolling (below) plus an LRU
   budget.

Both failure modes show up within a minute without the fixes (run1: Sway panic; run2/3: killed by the VRAM
watchdog; an earlier run: DeviceLost then exit 101).

## Caching: 2D PNG while moving, live 3D when settled

- **Choice.** The cache is 2D pre-rendered pictures, not a 3D model cache. A loaded model costs its textures in VRAM
  plus a load off the disc. A 384×480 PNG is 0.7 MB decoded and is read on a worker thread. Scrolling only ever
  needs the picture, so the select shows the PNG and asks for the live model once the card has stayed on one
  character/costume for 0.3 s (`SETTLE`), or right away on R2/L2/△ (below). The live model then replaces the PNG in
  place.
- **Budget (M1m's on-demand load + LRU).** At most `KEEP_MODELS` = 6 loaded characters (plus whatever is on screen)
  and 48 PNGs, least recently used dropped first. The loader thread works newest-ask-first and drops asks more than
  8 deep as stale.
- **Baking.** An off-screen preview (render layer 6, 384×480, the same `inspect::spawn_into` and camera as a card)
  draws each missing PNG. It reads the target back with `Readback::texture` and converts the Rgba16Float
  GS-encoded colour exactly as `inspect.wgsl` does (sRGB, alpha = a+1) to RGBA8. It refuses an empty picture.
  Hovered keys are drawn first, then every disc character × costume (140 on this disc). `HST_BAKE=all` also draws
  every mod costume, then quits.
- **Keying.** `<iso dir>/cache/previews/<boot file>-r<RENDER>/`. The boot file name (`SCUS_976.10`) comes from
  SYSTEM.CNF, so a different disc gets its own folder. Disc characters are `dNN-<costume>.png`. Mods are
  `m-<folder>-<newest mtime of mod.json/motions/costume glbs, hex>-<costume>.png`, so editing a mod draws it again.
  On start, any other `cache/previews/*` folder is deleted, which clears out a stale renderer version (bump
  `RENDER` when the preview look changes).
- **Mods ship their own.** A mod may carry `previews/<costume glb stem>.png`, which is used ahead of the cache and
  never baked. Added to ../opentennis-mods `standard/` §1, with `tools/rerig/previews.py` copying HST's bakes into
  `out/mods` and a README step (opentennis-mods 7d7043b). Checked: a scratch ISO dir with one mod carrying shipped previews
  baked the 140 disc pictures and none for the mod.

## Results

- **Second launch reads the cache.** Two plain 75 s launches on an empty cache: launch 1 baked 140, launch 2 baked
  0 (140 files, no panic). Own VRAM peaked at 2.07 GB and 1.85 GB; the menu alone is 1.45 GB.
- **`--shot` PNG vs live** (`HST_PREVIEW=png|live`, `HST_SELECT=0,5,9,2`, `--shot-at 6`): same framing, size,
  lighting and colour. 1.2% of the pixels differ, all inside the four cards, because the live idle motion is at a
  different frame (`context/b86/shots/`).
- **Stress runs.** run7: 175 s at full speed through disc characters, costumes and mods, own VRAM flat at about
  3.1 GB, 414 PNGs baked, no panic. It was stopped only by the watchdog's card-wide guard (other slots' processes).
  Final run (`HST_VRAM_GB=4`, 300 s asked): 272 s at full speed, no panic, 274 pictures baked. Own VRAM held at 3.1–3.5 GB from 40 s on, with one 1 s spike to 3.99 GB while the four cards, the LRU and a bake job overlapped. The watchdog stopped it at 272 s on the card-wide guard: the card was at 9.75 GB, the rest from other processes (a user `dist/linux/hst` and other slots), not growth in ours. A run with the whole card free was not done.
- **Stress stays on the select.** User report: the stress run reached other menus and changed settings. Under
  `HST_STRESS` the menu's own input system is off and stress forces `Screen::Chars` around every step. settings.txt
  is unchanged.

## User notes (2026-10-12)

- **Preview animations with voice lines.** R2/L2/△ already play win/loss/pose on a card, but no voice line plays
  with them in the port. This fits the cache: a press on a card still showing its PNG loads the live model straight
  away (`Pending`), and the motion starts as soon as the model spawns. A voice started there would play in sync, so
  it can go in the same spot. → **B86a**.
- **Modded characters still sometimes miss parts such as hair.** This was already **M1k** (re-report added there).
  Because the select now draws mods from cached PNGs, a fix must bump `RENDER`, or the old pictures stay.

## Not verified / not 1:1

- **No reference in the original.** The PNG cache, the 0.3 s settle and the budget are ours, not the original's (which has
  only the disc cast). They exist for 100+ characters with upscaled textures (menus are "similar, not exact",
  user 2026-10-11).
- **First bake waits a fixed time.** It waits 30 frames, then 4 frames per bake after that, before reading the target
  back. This is a heuristic for the pipelines compiling. The empty-picture check catches a too-early read: it skips
  the key for this run.
- **Old mod bakes are never pruned.** Bakes left behind by a mod's old stamp stay in the cache folder until `RENDER`
  changes.
- **Mod icons on the confirm screen** still use the dark placeholder card (not in this task's done-when).
- **Shot comparison.** It compares PNG against live in our build, not against PCSX2. The live previews' look was
  checked in B40b and B84.
- **`HST_BAKE=all` over all 92 mods** was not run end to end here. Mod bakes were checked through hovering: the
  stress run baked 274 mod pictures.
