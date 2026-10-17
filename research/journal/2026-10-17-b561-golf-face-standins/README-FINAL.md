# B561 — Fore!, Open Tee 2, Get a Grip mods: stand-ins for unbound face channels

B542's check (`oob_faces_follow_the_donor`) now also runs over every Fore!, Open Tee 2 and Get a Grip mod
(`mods.rs` `golf_faces_all`, costume 0), and asserts exactly the gaps listed below.

## Before

Fore: most lacked `doki_mouth`; Sam/Maya `sorrow_mouth`; Chaos/Ratchet `joy_eye`; Jak sorrow; Tiffany doki + sorrow;
Kamala anger + `doki_mouth`; Tbone and Louise most eyes; Z everything (its model has no face targets).
Open Tee 2: every one `doki_mouth` (face.json doki null: no motion shows a consistent surprise face).
Get a Grip: all bind but Helghast (a mask, face.json channels all null).

## Picks (by eye)

Fore, from zoomed renders of every target (check.py's face-sheet, all targets, head-only box):

| mod | channel ← target | why |
|---|---|---|
| Phoebe, Emma, Lin | doki_mouth ← sorrow_mouth | their open, rounded mouth |
| Brad | doki_mouth ← sorrow_mouth | parted grimace, his only open non-grin |
| Falcon | doki_mouth ← laugh_mouth | open round mouth |
| Regis, Allan | doki_mouth ← pain_mouth | jaw dropped |
| Misaki | doki_mouth ← su-_mouth | lips parted |
| Toni / Louise / Zeus | doki_mouth ← yo / ho / u | open O vowels |
| Hubert | doki_mouth ← vexing_mouth | his is wide open (Emma's is a closed frown, hence per character) |
| Sam, Maya | sorrow_mouth ← he_mouth | downturned pout / frown |
| Jak | sorrow_mouth ← vexing_mouth | flat grimace |
| Chaos, Ratchet | joy_eye ← blink_eye | closed happy eyes, like HST's joy eyes |
| Tiffany | doki ← vexing_eye1 / vexing_mouth; sorrow ← vexing_eye / anger_mouth1 | wider eyes, parted lips; droopy lids, pout |
| Kamala | anger_mouth ← sorrow_mouth; doki_mouth ← sexy_mouth | bared teeth; pursed O |
| Tbone | doki_mouth ← anger_mouth; sorrow_mouth ← vexing_mouth | open shout; grimace |

Plus B546's shared `doki_eye ← sorrow_eye` (already applied in game by `mods.rs`' fallback), now baked in too.

Open Tee 2 (whole-face textures, `out/textures/opentee2/face/faceNN_<letter>.png`, letters past a–d were never
packaged): doki = Mika e, Rio f, Alia e, Shu e, Julie e, Patricia c, Logan f, Holly f, Roger f, Carly c, Luke c,
Alice e, Malachy e, Frau Ada e, Lauryn c, Chad c, Gloria c (wide eyes and/or open mouth with brows that don't
frown; c is also their anger texture, which reads as surprise on those faces).

## How (opentennis-mods 7484c72)

- `rerig.py` `FACE_PICKS` (by mod id; `face_key` reads it from the source `out/models/<game>/…/pcNN_name_cNN.glb` or
  packaged `out/mods/<id>/model/` path), tried first for a channel the model lacks. Per character, not words: the same
  target name means different faces on different characters (`vexing_mouth`). `face_alias.py` applies it in place.
  So `mods.rs` is unchanged: rebuilt models carry the channel under its own name; installed v1 copies keep the old
  faces until they update.
- `opentee_data.py` `DOKI` letters; packaged face.json patched in place (doki channels + label, PNG linked into
  `face/`), as package.py's `golf()` would write them.
- Rebuilt: all 8 costumes of 23 Fore mods (face_alias rerun is a no-op; check.py passes), 17 Open Tee 2 face.json.
  `mod.json` `version` → 2 on those 40.

## Not verified / not 1:1

- All stand-ins are approximations by eye from static renders/textures, not the donor's own expression; several
  surprise faces share their anger (Open Tee 2 c, Tbone, Kamala's anger = her sorrow) or sorrow texture/target.
- Left without a target (asserted by the test): Z (no face targets), Helghast (a mask), Tbone and Louise's eyes
  (opaque glasses), Kamala anger_eye and Jak sorrow_eye (nothing glaring/sad), Open Tee 2 CJ, Montcalm, Mifune, Lee
  doki (every open mouth frowns): those stay neutral.
- Not looked at in a match or the viewer: presentation only (no sim/RNG/timing code touched); the test checks every
  track binds.
- Not uploaded: the rebuilt mods (version 2) are in `opentennis-mods/out/mods`; re-uploading to the store and pushing
  opentennis-mods are left to the human. B560 (hit cries) edits the same mods' voices and may bump the same
  versions; one re-upload covers both.
