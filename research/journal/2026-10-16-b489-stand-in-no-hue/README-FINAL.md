# B489 — online stand-ins without the hue shift

User 2026-10-16: a peer that declines a mod character's looks now sees the plain donor character.

- `hst-base/src/mods_online.rs`: `tint` (rotated every RGBA8 texture's channels, a 120° hue turn) removed with its
  import; module/`for_match` docs say the donor stands in as it is.
- `hst/src/play/mod_match.rs`: the stand-in branch just loads the donor (outfit, else costume 0); no recolour.
- `play/main_menu/online.rs`: the ask pop-up no longer promises "in other colours"; comments updated.
- No test covered the tint; `mods_online` (3) and `mod_match` (6) tests pass.

## Not verified / not 1:1
- Not looked at in a live online match (needs two peers, one without the mod); the change only removes a texture
  edit, the load path is the same donor load stock characters use. Online mods are our addition, nothing in the
  original to compare with.
