# 1: one-sided materials back-face culled

## Finding

- Camera placement matches the original. Hiding the `sclwall` props (`skip_sclwall.png`) showed that only the fence was in the way.
- The VU1 loop choice depends on material header +0x24 (p17 journal, 2-VU1-LIGHTING). A non-zero value takes the lit loop without culling. Zero culls back faces.
  - `mtl::Material.two_sided` already reads this flag for collision.
- Strip parity does not give triangle winding: with it, the ground lost random triangles. The winding comes from the third vertex's UV `w`, the same rule the collision walk orients by (`hst-sim` mesh.rs, checked bit-exact in tests/collision.rs). ≥ 0 keeps strip order; otherwise the order is reversed.

## Fix

- `hst-data` mdl.rs (rigid packets): each triangle is ordered by its UV `w`.
- `hst` gs.rs: `GsKey.cull = !two_sided` sets `cull_mode = Back`.
  - The shadow pass (`DEPTH_PREPASS`, which GsMaterial uses only for shadows) keeps both faces, as before. It has not been checked against the game's caster draw.
- Skinned characters still use parity. They don't go through GsMaterial.
- Test: `tests/winding.rs`. Court 1's draw triangles must face the way the collision list orients them. It fails if the UV-w rule is dropped.

## Evidence (context/b7/, git-ignored)

- `m19.png`: the original on court 1 (PCSX2).
- `s1_*.png`: ours before the fix (fence across the view).
- `cull_w.png`: ours after the fix (matches m19).
- `c2cmp.png`: stage 2 is unchanged. Stages 5, 10 and stage 1 singles also look sane.

## Notes

- PCSX2 copy 5 has swapped pad buttons: cross confirms and circle goes back, so `pick.py`-style circle presses fall back to the menu. Court select path: Fun Time Tennis → confirm → Select Court. RAM 0x422f90 holds the court index.
- `tools/check.sh` fails at hst-sim `volleys_launch_like_the_game` ("only 0 of 4 volleys exact"). HEAD without this change fails the same way, so the failure is not from B7.
