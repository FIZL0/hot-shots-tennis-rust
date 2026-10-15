# M1k — mod parts missing in the select's 3D previews

## Which mods, which parts

All 501 select previews were baked (`HST_MENU=chars HST_BAKE=all`), and the cards were compared on contact sheets
and per pixel against `--inspect` shots of one mod at a time (`--inspect MODDIR:C`; the costume suffix is new).

- **Fore! Zeus, costumes 4–7**: the whole gi (`jacket` and `pants_obi` primitives) was missing, along with most of the hair.
  The bake had 20050 covered pixels; it has 38127 now.
- **Most Out of Bounds mods** (Sophie, Jasmine, Sasha, Sonia, Kate, Anya, Gloria, Alex, Dino, Fernando, Suzuki…) and
  Fore! Hubert/Jak: hair tips, pigtails and racket strings were missing. These are BLEND materials with soft alpha.
  130 cards changed.
- Disc characters: no change. Their blended parts sit over an opaque draw, or there are none.
- `--inspect` (window): always whole.

## Why

The cause is in the draw, not in load, rerig or the opentennis-mods packaging. The baked PNG's RGB had the gi, but its
alpha did not. Every GS blend kept the destination alpha (`gs.rs` specialize: alpha `Zero·src + One·dst`).
A preview target is cleared to alpha −1, meaning "nothing drawn" (`inspect.wgsl`). A part drawn only by blended
passes therefore left −1 and showed as transparent on the card. That covers a glTF BLEND material, which is GS mode 25:
the Ge70 and Lt70 passes both blend. The window ignores alpha, so `--inspect` looked fine.
`inspect.wgsl` already expected "a blended one leaves 2a − 1": that was the plan in B40b1, but P17a's GS blend
state never wrote it.

Ruled out on the way:
- costume noise (with no mesh copy, the parts are still missing; the vertices don't move);
- late pipeline compiles (the readback was gated on the `PipelineCache` having nothing waiting, and parts were still missing);
- upscaled textures (Zeus ships none).

## Fix

- `gs.rs`: a blended draw's alpha is `src + dst·(1 − a)`. Over the −1 clear that leaves 2a − 1, so a part drawn
  only blended still shows. Over an opaque draw it stays ≥ 0. Only the preview cameras read the target's alpha
  (`CameraOutputMode::Write` REPLACE). The game's view already has arbitrary alpha from opaque draws, so a match
  draws the same.
- `previews.rs`: `RENDER` 1 → 3. M1l on main already took 2, so bakes from either branch are redrawn.
- `inspect.rs`: `--inspect MODDIR:C` shows costume C.
- `mod_match.rs`: main's M1l test change (Wendy instead of Emi) is applied as is. Repackaging made Emi's
  style-matched AI row equal her donor, so the old test failed on this branch.

After the fix, a full re-bake: every card on the three contact sheets draws whole. Images and work files are in
`context/m1k/`: `before_after.png` (Sophie and Zeus 5, before and after), `sheet6{0,1,2}.png`, `after_inspect.png`,
and `cmp.py` (per-pixel diff of two bake folders).

## Not verified / not 1:1

- **Edge colour on the cards**: semi-transparent texels, such as hair tips, now show at opacity 2a over the card,
  with their colour blended onto the clear's black. That is not exactly a. It looks right on the sheets, and the
  front end doesn't have to match.
- **`--inspect` for every mod**: only Zeus 5 and Sophie 0 were shot after the fix, and the earlier
  one-mod-at-a-time shots covered the rest. The window path reads no alpha, so the change can't affect it.
  A multi-mod `--inspect` draws the previews overlapping, so a single shot can't cover all mods.
- **Live select cards** (`HST_PREVIEW=live`): not shot separately. They use the same `spawn_into` target and
  shader as the bakes.
- **Bake-to-bake sway differences** (spring and noise parts: Sophie's pigtails, Louise, Dino, Phoebe's arm): the
  sway is sampled at whatever frame the readback lands on. This is cosmetic, not a missing part, so it's left as is.
