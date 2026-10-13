# B143 — costume icon hidden under a mod character's △ card

The select draw skipped the player tag + costume shirt only for disc characters' attribute cards
(`!(pl.card && modded.is_none())`), so a mod's card had the shirt over it. Now `!pl.card` for both.
`HST_MENU=mods-attrs` opens the custom page with 1P's △ card (for `--shot`).

Checked (`context/b143/`): `mods.png` (stats off: 1P tag + shirt shown), `mods-attrs.png` (stats on: hidden),
`mods-attrs-wide.png` (1600×600, same), `attrs.png` (disc character unchanged).

## Not verified / not 1:1
- No PCSX2 comparison: mod characters don't exist in the original; the disc character's behaviour is the reference.
