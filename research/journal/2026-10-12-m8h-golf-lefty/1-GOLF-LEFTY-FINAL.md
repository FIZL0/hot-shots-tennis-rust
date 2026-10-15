# Golf mods: glove hand = racket hand (FINAL)

The game draws a left-hander mirrored (whole rig, x scale −1: `shade.rs`, `gs.rs` mirror), so `hand: left` alone puts
the racket in the drawn left hand but moves the glove to the drawn right. ../opentennis-mods `tools/rerig/mirror.py` therefore
pre-mirrors a gloved golf mod's costumes (positions/normals/morphs reflected, L/R joint weights swapped, winding
flipped; twin-less joints get a `_mirror` copy) and `package.py` sets `hand: left` + `hand_why`. The game's mirror
brings the source's look back: text reads right (Jasmine's visor, Phoebe's "HSG FORE!"), glove and racket in the left.

- Glove test: mean texture colour of the two hands' vertices differs (> 25 |ΔRGB|). 62 of 72 golf mods are gloved →
  left. Right-handed (no differing hand, left to confirm): fore pc02 emma, pc03 sam, pc07 regis, pc11 z, pc22 ratchet,
  pc23 jak; oob pc15 alex; opentee pc05 cj; opentee2 pc13 malachy, pc14 frauada.
- Fix on the way: a twin-less spring joint kept its `spring` after its vertices moved to the copy (Jasmine 27 springs
  vs 18; `mods::tests::oob_hair_swings_and_settles`); the original now drops it. mods tests pass.
- Checked: `--inspect` shots of oob pc00 Jasmine and fore pc00 Phoebe beside Carol (context/m8h/): racket in the gloved
  left hand, no inside-out faces seen.

Not verified / not 1:1: a full match `--shot` of a mirrored mod mid-forehand (inspect pose only); opentee/opentee2 not
looked at in-game (same code path). Standard §6 `hand` and the opentennis-mods README note the rule.

## Fix: faces missing on the mirrored mods (same day)
The pre-mirror flipped the triangle winding as well as reflecting. The game already culls a mirrored rig's front
faces (gs.rs `GsKey.mirror`), so the one-sided parts (Phoebe's face, skin) were culled. mirror.py now keeps the source
winding. Checked with `--inspect` shots of fore_pc00_phoebe and oob_pc00_jasmine: face and skin draw, text reads
the right way, racket in the gloved hand. The select card now shows a mod's hand badge like a disc character's (the
source game line moved under it).

## Fix: mirrored mods inside-out in a match (same day)
The "keep the winding" fix above was only checked in `--inspect`, which never applied the mirrored-rig culling
(shade.rs `FixedLight` rigs skipped `key.mirror`), so the left-handed golf mods (oob Anya, LJ, …, every costume)
drew inside-out in a match. Reverted in opentennis-mods (mirror.py flips the winding again, mods repackaged), and
shade.rs now sets `key.mirror` on inspect rigs too (each frame until their materials load). Checked: match shots of
oob Anya/LJ c00 and c05 from behind and in front, `--inspect` LJ c05, disc lefties 6 and 11 in `--inspect`
(context/seethrough/). Preview `RENDER` 3 → 4 so every install rebakes its cards.
