# 1 — roster scan (agent continues once B40 and M1c land)

- `mods::list(Path::new("mods"))` → `Vec<Mod>` (name, costumes, donor, hand; stats via `mods::tparam`).
- Test `mods::tests::lists_the_custom_roster` (`HST_MODS=~/repos/HST-MODS/out/mods`): all 87 packaged mods are
  listed, each of Fore!, Get a Grip, Open Tee, Out of Bounds (`fore_`, `getagrip_`, `opentee_`, `oob_`) is
  present, and a folder with a bad `mod.json` is left out. Passes.

## Left to do
- In B40's select (`play/main_menu.rs`, not on main yet): a page/tab after the disc roster listing `mods::list`,
  each costume, name, `tparam` stats, portrait (rendered from the costume, or the donor's).
- Picking one puts the `Mod` + costume in the player slot for M1c's match setup; same costume-lock rules.
- Done check: pick one and start a match with it on court (needs M1c).

## Not verified / not 1:1
- Everything of the select page, portrait and match start: not built, B40's select and M1c don't exist yet.
- Remaster-only feature: no original to compare the roster against.
