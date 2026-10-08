# P17q: ball tornado exact port — findings (PART, agent continues)

Object: update 3b53c0 (vtable +0x3c), slow-mo in-between 3b58c0 (+0x40), draw 3b5ba0 (+0x48), messages 3b5230
(4 save / 5 restore uv times + flash, 0x17 off / 0x12 on, 6/0xc/0xe zero 0x5c..0x190).

## Latch (the P17j note "4 frames after a serve hit" was a misread)
- The manager's per-player records (mgr +0x758 flag, +0x75c timing offset = player +0x3fa0, +0x760 grade,
  +0x761 branch) are posted at the contact *search* (2..8 frames before contact in foot_s05.bin), not the hit.
- The "ball away" flag mgr +0x750 = hit count 0x423060 changed (and last hitter != -1): the contact frame itself
  (ball velocity changes on that frame in foot_s05.bin: 150, 229, 578, 716, 1207, 1272).
- So the port's start on the contact frame is already the right frame. Remaining gap: the latch comes from the
  search with offset not 999 (dive, 3ec4=-3) / 9999 (miss swing): a dive hit starts no tornado unless an earlier
  search's latch is still pending. Port: latch on a player's `contact` / `serving.swing` becoming Some in
  `play/tornado.rs` (child module sees Game's private fields), start on `hit_effect` only if latched; clear the
  latch at the point reset (phase Post).

## Flash quad: draws nothing in retail
- Runs when the latched record's grade (+0x760) == 1; quad = centre ± A·s ± B·s with s = +0xd0, colour +0xe0..
- +0xd0, +0xd4 (s growth) and the colour/fades are never written (zero after the reset clear); foot_s05.bin frames
  1207..1214 with the flash on show all zero → zero-area, alpha-0 quad. Nothing to port; document in tornado.rs.

## Slow-motion in-between (3b58c0), only in slow-mo frames (0x2ef090 on: cut-away/replay slow-mo, P0b4d)
- frac = 0x2ef094 (1.0 when off). Restores saved uv times + flash first.
- scale = madd(add(0, t), speed, frac) if t < end else t (passed to the model; t itself unchanged).
- bounces >= 1 → off; else matrix as in tick; uv time = madd(add(0, prev), frac, sub(cur, prev)) with the
  saved uv times (anim +0x1c prev, +0x20 cur).
- if end <= scale: alpha = div(mul(128, sub(cvt(fade), frac)), cvt(30)), floored at exe 0x416658 (= 0);
  material alpha = alpha/128.
- The port has no slow motion yet: port as `Tornado::between` in hst-sim; verifying needs a slow-mo capture
  (idea: poke 0x2ef090 struct: +0=1, +8=+0xc=3.0, +0x1c=0, +0x24=+0x28=3 → constant ×4 slow-mo; record tornado
  0x100 bytes + 0x2ef094 each vsync).

## Left
- latch in play/tornado.rs; `between` in sim + capture test; look check (screenshot vs --shot).
