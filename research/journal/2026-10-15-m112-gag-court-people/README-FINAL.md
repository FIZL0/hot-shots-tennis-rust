# M112 — Get a Grip court mods without their people

User 2026-10-15: Get a Grip's chibi spectators/NPCs look out of place next to HST's cast; drop them on every
Get a Grip court, keep the court, umpire's chair and props.

## What counted as a person
- The gallery: `export_getagrip_gallery.py` (M8f7) wrote `npcs.json` + `npcs/` (billboard cards, crowd voices) on
  all 16 courts. Script removed from ../opentennis-mods; the court exporter deletes a leftover `npcs.json`/`npcs/`.
- Gimmicks that are people, found by their `gi_<Name>_face*.gim` textures: c01 Shin, c02 students (never placed),
  c06 fisherman, c07 clown and ClownCannon (the cannon's model carries the clown), c08 driver, c13 walker and RC boy,
  c14 Karl, batter, cheerleaders, man on the bank, c15 ninja, c17 window cleaner (`spaceman`), c18 master monk,
  c20 tightrope walker.
- Placed (haichi) figures: c09's line judges (`ob15/16_umpire0N`, Suzuki body) and the cameramen of c09/c16
  (`ob09/ob05_camera01`, `alan` body); c09/c16's whole-stand crowd `cNN_gall00` was already skipped.
- Kept: animals, vehicles, c16's studio camera rig (`gi04_Camera`, no figure), the umpire's chairs, c09's empty seats.
All in `PEOPLE` in `tools/psp2gltf/export_getagrip_courts.py`. Re-exported all 16: no node of those names in any
court/gimmick/sky glb, no `npcs*` left (script check in the session).

## Game
`mods_court::read` ignores `npcs.json` on a pack whose source is Get a Grip (packs built before this). The court-mod
match test now asserts the Get a Grip court has no people and checks the people pipeline with two stand-ins
(the mod's sky.glb, a four-sample wav). Shot on c06 Tennis Fields Club (`context/m112/c06.png`): no fisherman,
no gallery cards; the two figures behind the far baseline are HST's own Kar walkers (same spots on the
disc court, `context/m112/kar.png`).

## Not verified / not 1:1
- Packs installed before this keep the human gimmicks inside their court.glb/gimmick.glb (only `npcs.json` is
  ignored at runtime); re-installing the re-exported pack removes them. Reason: no node-name filtering of other
  modders' glbs at runtime.
- `previews/court.png` in out/mods are older renders (may show people); not regenerated here.
- ClownCannon dropped whole (its clown is part of the model, not a separate node): a judgement call.
