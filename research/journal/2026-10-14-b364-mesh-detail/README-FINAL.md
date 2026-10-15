# B364 — Mesh detail graphics setting

Settings → Graphics → **Mesh** (25 / 50 / 75 / 100 %, default 100) lets a weak GPU draw the players, their rackets
and the court figures (umpire, ball kids, crowd) with fewer triangles. It matters most for detailed mod characters.
Also `--mesh N` (10–100) and `mesh = N` in settings.txt. It isn't part of a preset; changing a preset keeps it.

## How it works (`crates/hst/src/lod.rs`)
- Quadric-error edge collapse (Garland–Heckbert, area-weighted quadrics) that rewrites **only the index buffer**.
  The vertex buffers stay as they are, so morph targets (faces) and noise.rs's vertex indices remain valid.
- The disc splits meshes into packets with duplicated vertices. Vertices with the same position and the same
  values for every other attribute are welded first (`keys`), so packet edges aren't mistaken for borders.
- Kept fixed: border and non-manifold edges (the seams between material parts, so parts never gape apart) and
  morph-moved vertices (`locked`: the face keeps every expression). UV seams aren't fixed: a collapse moves
  every copy of a seam vertex onto the matching copy of the target vertex.
- A collapse is refused when it fails the link condition, joins vertices skinned too differently (`SKIN_APART`,
  which stops skirts tearing in motion), flips or slivers a triangle (also against the triangle's original
  facing), turns a triangle's texture over, or costs more than the error bound `MOVE` (1 % of the part's size,
  scaled by how far below 100 % the setting is; this keeps fingers and shoe toes from turning into spikes).
- Runs at load in `character::parts_at` (disc characters and NPCs), the disc racket, and `mods.rs` (mod parts +
  racket.glb). Effects use `skinned_parts_whole` and are never reduced. About 10 ms per character.

## Numbers (triangles; `lod::tests::disc_and_mod_characters_reduce`)
| model | 100 % | 50 % | 25 % |
|---|---|---|---|
| pc00 (disc) | 2489 | 1539 | 1169 |
| pc06 (disc) | 2494 | 1348 | 1112 |
| Get a Grip Emi (mod) | 628 | 530 | 504 |
| Fore! Phoebe (mod) | 2915 | 2007 | 1733 |

The numbers reach the target only where the error bound allows. Low-poly models stop early, so 25 % means "as
low as it can go without visibly breaking".

## Checked
- Unit tests: a closed bumpy sphere halves and stays closed, without flipped triangles; a UV-seamed sphere
  reduces across its seam with every copy kept consistent (a mutation that sent copies to the wrong side was
  caught) and a locked vertex stays put; disc and mod characters keep their borders and grow no fins.
  Graphics settings round-trip/preset/flag tests; options pages fit. `tools/check.sh -p hst`: 345 passed.
- By eye (`tools/shot.sh … --mesh 100|50|25`, context/b364/run3/): Phoebe, pc00 and Get a Grip at 25 % keep
  hair, skirt, shoes and hands. The only visible loss is slightly simpler fingers and shoe toes on the low-poly
  Get a Grip model. Earlier iterations fixed unwelded packets, holes in skirts, spiky fingers and texture folds
  on shoes.

## Not verified / not 1:1
- The whole feature is remaster-only with no counterpart in the original, so there's nothing to match. At the
  default 100 % meshes are untouched, so matches look and play as before. The feature doesn't touch the sim.
- Quality was checked by eye on three characters in one pose, not in motion through a whole match, and not on
  every crowd figure.
- Vertex buffers aren't shrunk (only the triangles drawn), so GPU memory doesn't drop.
- Court props, the court itself and effects aren't reduced.
- A change applies to characters loaded afterwards (the next match), not to the ones on screen.
- The Mesh row in the options menu was checked through the options tests (every page renders, pages fit at any
  aspect ratio), not with a screenshot.
