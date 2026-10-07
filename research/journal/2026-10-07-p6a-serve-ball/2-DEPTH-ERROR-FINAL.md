# P6b — serve depth error from contact height

Verified bit-exact on all 36 serves of match_s05 (`serves_scatter_like_the_game`; 11 land off their aim).

## What the game does

At the swing (serve branch of the swing setup):

- Depth error (+0x3ecc, tenths of a metre, + deep) = depth bias of the contact frame + contact-height term.
  - Bias: an int table beside each timing table (strong +0x1554 next to grades +0x1644; weak/underhand +0x1684
    next to +0x1774), indexed like the grades (offset + 8). Slot 5, all four players:
    `-8 -6 -4 -2 0×9 2 4 … 16`.
  - Height term: off = contact height − ideal (overhand +0x13d0 / underhand +0x13dc; stored +0x3f9c);
    cm = (int)(|off|·100)/10·10; past threshold T (+0x1364 = TParam Strk GH) by n: n<0 → 0, n < S
    (+0x1368 = Strk NH) → n/10, else S/10 + (n−S)/10·2; ×6 (serves), signed by off. In match_s05 every
    contact is within 40 cm, so the term is always 0 there (checked via +0x3f9c bit-exact, the term by unit test).
- Serve launch: a clean grade (1/2) zeroes it; clamped ±10 (the clamp table has −10..0 for a flag set only by
  non-serve aims). Sideways int +0x3ed0 = the aim's centre-line nudge (+0x3ed4: |aim x| ≤ 1 with stick x = 0 →
  ±10/±5 from two coin flips), clamped ±10.
- side = (+0x3ed0/10)/2 + stick miss (+0x3f10); depth = +0x3ecc/10 + stick miss (+0x3f18). Strong toss and
  depth < 0: depth × TParam column 54 (サーブ手前ブレ倍率, +0x13a8 = 3).
- Launcher: (1.5·side, 0, 1.5·depth) turned into the frame (across = up × dir, up, ahead) with dir = aim − contact
  in xz (FPU normalize, VU0 cross/normalize/transform); the ball flies to aim + that.
- `dw1` selector: +0x3ed8 = −5 for a weak toss → −0.5 × min(Serv POW, LOW POW)/Serv POW (+0x12e4, +0x1398 =
  TParam col 12 / col 50); < −0.2 picks dw1. Every character in slot 5 has LOW POW ≥ Serv POW → always dw1.

## Port

`serve::depth_error`, `serve::dw1`, `serve::scatter`, `serve::Miss`; `target` returns the miss (and the nudge)
instead of a world-space scatter (P6a's 2/3 scatter was missing the ×1.5, the rotation, the bias and the ×3).
play.rs reads the stats from TParam; the bias table is character 0's measured one, like the grades.

## Open

- The stick part of the strong-toss miss (+0x3f10/+0x3f18) follows the decompile's arithmetic but is checked
  only through its recorded values (the game's stick vector at the press isn't in the recording); same for the
  centre-line nudge (no case in match_s05).
