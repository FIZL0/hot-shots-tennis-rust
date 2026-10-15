# B241: Falcon's shoe laces off his boots

**Cause: the mod (open-tennis-mods `tools/rerig/mirror.py`), not our renderer.**

Falcon (fore_pc09) is left-handed, so his costumes ship pre-mirrored. His lace bows (Material #2237, 24 verts) are
skinned 100% to two extra joints, `node78` (child of `Bip01 L Calf`) and `node84` (child of `R Calf`). Fore's own
export puts both joints *at the world origin* (local offset ≈ 0.47 m down the calf), bind = rest, and no Fore motion
animates them, so they ride the calves rigidly. That's fine on its own.

mirror.py called any joint with |bind x| < 5 mm and no L/R name twin "midline" (its own twin). Both lace joints are at
x = 0, so each twinned itself: the vertices were reflected to the other foot but kept their joint, and each boot's
laces followed the opposite leg (with the default viewer pose they hung between the feet, seen from behind).

Fix (open-tennis-mods ab53ea3): midline only if the whole parent chain is on the midline. The lace joints then pair by
position + twin parent (node78 ↔ node84). The same bug hit three oob mods, fixed by the same change: Jasmine `h_mimi`
(under `R_ering02`), Kate `udewa` (under `Bip01RForearmRoll`, now gets a mirrored copy), S. Maruyama `pl1/pl2`
(under the feet). Regenerated only the 21 costumes whose mirror output changed (old vs new mirror.py on the same
out/rerig input: Falcon c00–c03, Jasmine/Kate/S. Maruyama c00–c05); Falcon c04–c07 have no lace joints. check.py
passes on them (only the old face-channel warnings).

Check: `tools/shot.sh … --mod mods/hst-mods/fore_pc09_falcon --radius 2.5`: before, the bows floated between the
boots; after, they sit on the boot fronts (shots in `context/b241/`).

## Not verified / not 1:1
- No match `--shot`: the viewer close-up runs the same skinning path, and the laces are a few pixels at match distance.
- Not compared with Fore running in an emulator; the fix only puts each lace back on the leg it was modelled on.
- The mod store (Hugging Face) copies of these four mods still hold the old costumes until they are re-uploaded
  (M17 upload flow).
