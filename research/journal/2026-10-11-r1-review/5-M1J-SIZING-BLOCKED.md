# R1 → M1j mod sizing: implemented, done-when half met (needs a ruling)

Implemented in ../open-tennis-mods (`tools/rerig/sizing.py`): every game is sized at packaging, with one factor per source
game taken from its own height data, so each game's relative heights are kept. Fore has no height data, so it gets
model height × 0.9685. Every `mod.json` records a `size_why`.

Bip01 rest heights (measured with `sizing.model_height` on out/mods and on the disc rigs):
- HST women 0.759–0.946 (Carol … Lola), men 0.460 (JJ) – 0.928 (Will).
- Mods: the tallest are 0.95 (fore_pc11_z, getagrip_pc10_schneider, oob_pc09_lj, opentee*_pc07_logan), level with
  Lola and Will. Most sit in 0.58–0.95.
- Out of range for their sex: many mod women at 0.62–0.73, below HST's shortest woman (Carol 0.759). This is what
  one factor per game gives (e.g. OOB 0.8264, from its cm heights). Fore's one-off bodies are shorter still:
  fore_pc02_emma 0.264, fore_pc07_regis 0.217 (the bulldog), fore_pc22_ratchet 0.471. HST's JJ (0.46) is the only
  disc precedent.

Shots (`--inspect` draws every preview on one spot at one scale, so overlaps compare heights directly):
context/shots/r1-m1j/tallest2.png (disc 0, 5, 9, 13 + z, logan, lj, regis), pair.png (all 14 disc | z, logan, lj,
schneider). Feet are level, and the tallest mods' heads come within a cap's height of the tallest disc heads: in
proportion by eye.

## Not verified / not 1:1
- "Within HST's range for its sex" fails for the mod women at 0.62–0.73 and for the three Fore chibis. That can't
  be fixed without dropping "keep each source game's relative heights". A game's female spread runs lower than
  HST's, so any per-sex clamp squashes it.
- In the shots, heights were compared by eye only (no pixel measure). Fore's factor is not from data.

Unblock: the user picks one. (a) Keep the per-game factor and accept women down to ~0.62 and the chibis, like
JJ. Then tick M1j as is. (b) Clamp to HST's per-sex range, flattening each game's spread. Then
`sizing.py` gets a per-sex floor and the mods are repackaged.
