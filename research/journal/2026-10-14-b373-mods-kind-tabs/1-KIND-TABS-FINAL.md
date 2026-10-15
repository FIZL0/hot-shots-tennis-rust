# B373 — Mods kind tabs (FINAL)

Spec: `plan/B372.md` § Kinds.

## What landed
- `modstore::Kind` + `Meta.kind` (`"kind"` in meta.json: `character` / `court` / `umpire` / `textures`; absent =
  character, and a character's meta is written without it, so old entries and uploads are unchanged; an unknown word
  reads as `Other` and shows on no tab). M5a/M5c reuse this field (`textures` is M5a's word).
- `play/main_menu/mods_kind.rs`: the kind row under the page tabs on all four Mods pages — L2/R2 glyph caps
  (keyboard 1 / 3, the keys over Q / E; ZL/ZR, LT/RT per pad style), Characters · Courts · Umpires · Texture packs
  with counts, the current one gold, the online note on the right (shrinks, never squeezed, at 4:3).
- Every page lists only the chosen kind (Browse items, an uploader's Download all, By-author list, My uploads;
  Installed/Upload: installed mods are characters). Courts, Umpires and Texture packs have no store support yet:
  their tabs have zero rows and show the empty state ("Courts and umpires arrive with M8f." / "Texture packs arrive
  with M5: until then, install packs by hand in mods/texture-replacements/."), and their counts are 0 even if the
  store held such an entry (it couldn't be installed correctly yet). There only Q/E, L2/R2 and ○ act; the legend
  says so.
- Input: `Press.kind_prev/kind_next` (L2/R2, 1/3); `pointer::Act::Kind(i)` taps; legend `Act::Kind` "Kind".
- `--shot` sample: `HST_MENU=browsemods|addmods|uploadmods HST_KIND=0..3`.
- Tests: `mods_kind::tabs_go_round_and_say_what_is_coming`, `mods_screen::kind_tabs_filter_every_page` (all four
  pages, pad/keys/tap, empty-state rows, legend), `modstore::reads_kind`, legend test extended with an empty kind
  and L2/R2 handled-vs-listed.
- Shots (`context/B373/`): browse/install/upload at 1280×720 and 960×720 for each kind: row, note and empty state lay
  out at both.

## Parallel note
B372 (picture grid, rail, detail panel) was being rewritten in another slot at the same time. To keep the merge
small, all B373 code is in `mods_kind.rs`; `mods_screen.rs` only got a `kind` field, filters, the row call in
`layout`, `boxes` moved down by `mods_kind::ROW_H`, and the empty-kind early path in `mods_step`. When B372's
grid lands, its grid/rail/detail should draw under the same row and read `View::kind` / `empty_kind()`.

## Not verified / not 1:1
- Per-kind rails, tiles and detail panels (court surface / disc layout / people / sound; umpire ▶ sample calls and
  calls covered; texture pack split tile, before/after drag, texture count): not built. Nothing can reach them yet —
  the store carries no courts, umpires or packs, so those tabs are the empty state, and the data they need (surface,
  call counts, before/after pictures) isn't in `Meta` yet. Noted on M5d and M8f to build them with store support.
- Character rail/tiles/detail are B372's; this task kept the current list (B372 not merged when this ran).
- L2/R2 on a real pad not tried by hand (tested through `Press`); pad glyphs checked in the glyph test, not on a shot.
- Menus aren't 1:1 with the original by design (no original screen to compare).
