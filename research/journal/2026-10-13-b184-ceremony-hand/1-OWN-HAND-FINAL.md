# B184: Carol's ceremony mirrored

## What the original does (decompile, the result object's setup and its racket placement)
- Per player the setup copies the **character record** (0x118 bytes, TParam) and tests its hand word: `== 2`
  (left) → matrix x = −1, then turned π about y (a mirror in z), the model's left-hand flag (+0x135) set, the
  object's hand (+0x1e0) = −1; else identity, flag 0, hand +1. The flag only flips the GS cull bit (the model's
  draw XORs 0x20 into the packet when it is set), so it is not a second mirror.
- The select's hand toggle is never read: a lefty character is z-mirrored in the ceremony whichever hand she
  played with. Confirmed on slot 3 (P1 Carol toggled to right-handed, +0x12b4 = +1 in the match): in the
  ceremony her matrix is diag(1, 1, −1), flag 1, hand −1 (`research/b184_slot3.py`, log in `context/b184/s3`).
- The racket is a model of its own. Each tick it is placed at the racket joint's world matrix with its x row
  multiplied by that hand, i.e. mirrored back in its own x.

## The port before
`ceremony::pose` mirrored by `players[i].hand`, the hand **played** (after the toggle). A Carol or Will switched
to right-handed (the user's own saves have Carol that way) stood unmirrored in her `_n` motions, which are made
for the mirror, so the air guitar came out mirrored against the original. The racket was a child of the joint
and kept its mirror.

## Fix
`ceremony::pose`: the z mirror comes from `Game::strong_hand[i]` (the character's or mod's own hand, before the
toggle), and the racket's parts get `scale.x` = that hand. Rigs are spawned fresh for each match, so the
racket's scale doesn't carry over. This holds for every character (and mods, whose own hand is the same field).

## Faces
The user also saw no facial animation in the ceremony. Every ceremony motion has a `.MOR`/`.UVA`; the port
doesn't play them. Filed as **B184a**.

## Not verified / not 1:1
- No side-by-side screenshot of Carol's own air guitar in the original: the user's saves have no match where a
  Carol (or Will) side wins and is shown (slot 5 is Ashley/… bots; slot 3/4's human side lost both times the
  match end was poked). Poking slot 5's character ids to Carol (`research/b184_carol.py`, `context/b184/orig*`)
  didn't help either: those figures still face away, so the motions (and models) are loaded before the poke. The fix rests on the
  decompile plus slot 3's RAM (matrix, flag and hand of a toggled Carol). At the user's request, the decompile
  rule was preferred over chasing a capture.
- The match's own racket for a left-hander may have the same unmirrored-racket rule in the original (the port
  parents it to the joint, mirrored). The match's racket placement wasn't checked; only the ceremony's.
