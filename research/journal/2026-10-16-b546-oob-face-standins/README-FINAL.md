# B546 — Out of Bounds mods: stand-ins for the face channels their models lack

Picked by eye from the source face sheets (`opentennis-mods/out/models/oob/_face_check/pcNN_*.png`), for the
channels B542's table left unbound. HST's doki is surprise/flustered (wide eyes, open mouth), sorrow a down
or pouting mouth, anger bared teeth.

| mod | channel | stand-in | why |
|---|---|---|---|
| Jasmine | sorrow_mouth | `puku_mouth` | sulky pout, nothing sadder |
| Sophie | doki_eye / doki_mouth | `sorrow_eye` / `chu_mouth` | no wide-eye target: raised brows; small rounded lips |
| Bjorn | anger / sorrow / doki mouth | `i_mouth` / `n_mouth` / `a_mouth` | bared teeth / flat closed / wide open |
| Fernando | doki_eye / doki_mouth | `sad_eye` / `talk_mouth1` | brows up over the sunglasses; slightly open |
| Felipe | anger_mouth / doki_eye / doki_mouth | `sad_mouth2` / `sad_eye` / `talk_mouth` | bared-teeth grimace; brows; open |
| Bloom, LJ, Dino | doki_eye | `sorrow_eye` | raised brows (LJ, Dino behind sunglasses) |
| Suzuki | doki_eye | `Lbrow_up` + `Rbrow_up` (summed) | both brows up: surprise, behind sunglasses |
| Kate | sorrow_mouth / doki_mouth | `pucker_mouth` / `a_mouth` | pout / wide open |
| Anya | sorrow_mouth | `kiss_mouth` | pursed lips, the only closed non-smile mouth |
| Alex | doki_mouth | `a_mouth` | wide open |
| Fernando | blink_eye | none | opaque sunglasses and no eye target: a blink would not show |

## How

- Words (any model lacking the channel; tried after the existing ones): anger `i`, sorrow `puku pucker n`, doki
  `a talk chu`, in rerig.py `FACE_WORDS` and `mods.rs` `FACE_WORDS`. None is an HST track word, so donor tracks bind
  as before.
- Whole-target names, last resort, `FACE_NAMES` in both: doki_eye ← `Lbrow_up+Rbrow_up`, `sorrow_eye`, `sad_eye`;
  anger_mouth ← `sad_mouth2`; sorrow_mouth ← `kiss_mouth`. Words can't say these (another channel's word or variant,
  or HST's own `kiss`, whose tracks must not move to sorrow). The brow pair comes first: Suzuki's packed model already
  has a `sorrow_eye` alias (his pain squint) that would otherwise win. `mods.rs` skips `+` sums (a rebuilt model has
  the sum under `doki_eye` itself).
- Rebuild: `tools/rerig/face_alias.py` (new, opentennis-mods a6cd9ec) appends the missing aliases to the packaged
  `out/mods/oob_*/model/*.glb` in place, same as a fresh rerig, without rerigging/mirroring/repackaging (a full
  `package.py` rewrites the manifests). Added exactly the table above on every costume; `check.py` passes on all;
  Suzuki's `doki_eye` = Lbrow_up + Rbrow_up exactly. `mod.json` `version` 1 → 2 for the 12 changed mods
  (Jasmine, Sophie, Bjorn, Fernando, Felipe, Bloom, LJ, Kate, Dino, Anya, Suzuki, Alex).
- Tests: `oob_faces_all` now asserts every donor face binds on every OOB mod but Fernando's blink;
  `sophie_smiles_and_winks` expects nothing missing; the fallback unit test covers Sophie's doki.

## Not verified / not 1:1

- All stand-ins are approximations by eye from the static source sheets, not the donor's own expression (no OOB face
  matches HST's doki exactly); Sophie, Bloom and Dino's surprised eyes are the same target as their sad eyes.
- Fernando still has no blink (nothing to bind; invisible behind his sunglasses anyway).
- Not looked at in a match or the viewer: face binding is presentation only (no sim/RNG/timing code touched), and the
  tests check every track binds.
- The face-sheet crop cuts above Suzuki's brows; his doki_eye was checked numerically (exact L+R sum), the brows
  themselves on the source sheet.
- Not uploaded: the rebuilt mods (version 2) are in `opentennis-mods/out/mods`; re-uploading to the store and
  pushing opentennis-mods is left to the human. Installed v1 copies already get all but Suzuki's brows through the
  `mods.rs` fallback.
