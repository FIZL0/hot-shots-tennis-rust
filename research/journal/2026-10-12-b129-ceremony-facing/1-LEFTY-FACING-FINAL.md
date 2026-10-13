# B129: winners facing the wrong way in the ceremony

## Cause
Only **left-handers** were affected, on either team or end. In the original, the ceremony setup (the result object's
per-player matrix) starts from identity. For a lefty (character record hand word = 2) it mirrors x and then turns π
about y, which comes out as a **mirror in z**. The far side is turned π again on top of that. The port used the
match's x mirror (`scale.x = hand`), so every lefty in the ceremony faced the other way. The lefty characters'
ceremony motions (Carol 6 and Will 11, the `_n` files) are made for the z mirror, so in the port Carol and Will
stood with their backs to the camera. That was the "sometimes".

How it was checked in the original (`research/b129_lefty.py`, slot 5, PCSX2 copy 1). The result object's matrices
(+0x2230 + 0x40·i):
- run1, all righty: near p0 = diag(1,1,1) at (0.75, 0, −6.4); far p1 = diag(−1,1,−1). These match the port's marks.
- run2, the hand word of p0's and p1's characters poked to 2: near p0 = diag(1,1,−1), far p1 = diag(−1,1,1),
  hand −1. The screenshot shows the poked lefty (character 0, whose motion is made for a righty) with her back to
  the camera.

## Fix
In `ceremony::pose`, `tf.scale = (1, 1, hand)` under the same rotation (identity, or π at the far end). This gives
exactly the original's matrices for both ends. Port shots (`context/b129/port/`): Carol + Will before (backs to the
camera) and after (facing it); the poked lefty after (back to the camera, as in the original).

Also fixed from the same decompile (the motion loader): players 2/3 load the lost motion pair (`a`/`b`) opposite to
their **partner's** (player 0/1), not their own character's flipped. The `bots_doubles` test expectation had copied
the old rule and was corrected; a new assertion pins the rule.

## Not verified / not 1:1
- A native lefty (Carol/Will) in the original's ceremony wasn't captured. No save state has one in a bot-only match,
  so the poke was done on character 0. The matrix rule is the same code path (decompile + RAM), and the `_n` motions
  facing the camera under it in the port is consistent with that.
- The lost a/b partner rule comes from the decompile only; no capture of a doubles loss with mixed tables.
