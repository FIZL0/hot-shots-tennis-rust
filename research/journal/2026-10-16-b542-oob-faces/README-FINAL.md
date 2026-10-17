# B542 — Out of Bounds mods' faces in a match

## What was lost, and where

Not in the export: every packaged OOB costume (`opentennis-mods/out/mods/oob_*`) carries its `face` morph mesh with
sane deltas (Sophie's blink 3 cm, 230 vertices) and the §4a channels rerig.py aliased; a head close-up of the
packaged, rerigged and pre-mirrored Sophie with each target at 1 (scratch copy of `check.py --face` cropped by the
posed face, since the packaged meshes are in bind space, not world) shows blink, smile, anger and sorrow correctly.
The loader and renderer were fine too: the face part is morphed, `MorphWeights` reach 1, blinks render in the viewer.

The loss was in **binding the donor's `.MOR` tracks to the mod's targets**: the loader bound a track only to a
target of exactly its name. HST characters' faces use more than the nine standard channels (dumped per disc
character): pc02/04/09/12 `smile_eye`/`smile_mouth`, pc03 `niya_*`, pc05 `tukare_*`, pc06 `joy2_mouth` + `wink_eye`,
pc08 `smile_mouth` + `wink_eye`, pc09 `sorrow_eye2`, pc10 `kiss_mouth` + `smile_mouth` + `wink_eye`, pc12 `wink_eye`.
rerig.py only fills the nine standard ones, so a mod whose donor uses the extras dropped those tracks. Sophie's donor
(6) keeps its `joy_mouth` at 0 and smiles with `joy2_mouth`, so her win reactions (0x2c, 0x2e) and select pose
(0x40) lost the smile and the wink; with doki missing in her model too, about everything but blinks, anger and
sorrow stayed blank.

Second, smaller: a face's length (the face clock's wrap) was taken only over the tracks that bound, so a mod missing
a channel could end or loop a face early. Disc 7 and 13 have no blink target and their blink tracks never counted, so
the right rule is "the tracks the donor's own model binds".

## Fix (crates/hst-base)

- `mods::face_target`: exact name after the object prefix as before; else the track's expression word picks HST's
  channel (rerig.py's `face_word` + `FACE_WORDS`, ported, plus `niya` → joy), bound to the mod's `<channel>_<part>`,
  else to the mod's own best same-part target by that channel's words (lowest variant, never one half of the face).
  `joy2_mouth` → `joy_mouth`, `wink_eye` → `blink_eye` (a wink shows as both eyes shut), `smile_eye` → `joy_eye`.
  `CharacterData::morph_target` (the ceremony's faces) uses it too; disc names keep their prefix, so disc characters
  bind exactly as before.
- `Face::weights`: where two tracks drive one target (Sophie's `joy_mouth` and `joy2_mouth`) the stronger shows; a
  target driven once is written as before (disc unchanged). `animate` and the ceremony use it.
- `character::face_counted` / `disc_motions_counted`: a mod's face length is over the tracks its donor's model binds
  (`disc_morph_names`); disc characters still use their own bound tracks.
- Tests: `donor_tracks_fall_back_to_the_nearest_expression` (word parse, fallbacks, combine), `sophie_smiles_and_winks`
  and `oob_faces_all` (every OOB mod in `mods/hst-mods`, ~5 s, skipped without them: every donor track with any
  expression in the mod binds, every face as long as the donor's; `--nocapture` prints the table below).

## Per character (costume 0; donor in brackets)

Gained by the fallback: **Sophie** (6: smile + wink in the win reactions and select pose), **Kate** (12: wink),
**Gloria** (12: `smile_eye`). Face length now the donor's for all of them.
Still unbound, because the source model has no expression with a matching word (rerig's table): 

| mod | no target for |
|---|---|
| pc00 Jasmine (0) | sorrow_mouth |
| pc02 Sophie (6) | doki_eye, doki_mouth |
| pc03 Bjorn (1) | anger_mouth, doki_mouth, sorrow_mouth |
| pc05 Fernando (11) | blink_eye, doki_eye, doki_mouth |
| pc07 Felipe (11) | anger_mouth, doki_eye, doki_mouth |
| pc08 Bloom, pc09 LJ, pc11 Dino (11), pc13 Suzuki (1) | doki_eye |
| pc10 Kate (12) | doki_mouth, sorrow_mouth |
| pc12 Anya (8) | sorrow_mouth |
| pc15 Alex (0) | doki_mouth |
| pc01 Nick, pc04 Sasha, pc06 Sonia, pc14 S. Maruyama, pc16 Gloria | — |

Choosing stand-ins for those (vowel/talk/pucker mouths, brow targets) needs a look at each face, so it's task B543,
not a guess here.

## Checked

Viewer `--mod mods/hst-mods/oob_pc02_sophie` vs `--character 6` (screens in `context/b542/run1/`): the idle (0) blinks
on her every 2 s loop; the win reaction 0x2e now smiles with closed eyes like the donor's grin + wink.

## Not verified / not 1:1

- A wink plays as a full blink on mods without a wink target (Sophie, Kate), a stand-in, not the donor's one-eye wink.
- Not looked at in a full match: the viewer plays the same `animate` path and faces as the match, and no sim, RNG or
  timing code changed (presentation only, per PLAN).
- The ceremony's faces (other characters' files on a mod) still take their length from the tracks the mod binds;
  it has no donor model at hand. Left as it was.
- No change to opentennis-mods (rerig.py/oob2gltf/notes): the export was fine and the loader fix covers installed
  mods without a rebuild or re-upload.
