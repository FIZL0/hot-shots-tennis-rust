# N2 — the ball in the server's hand

How the original places the held ball (player update, per serve mode at player +0x3fa6):
- Mode 0, stance (motion 0x20): the character's `sh_pcNN_serve_ad00_ball` track (one `ball` position track) at the motion's sampled time (+0x38), turned by the player's rows (+0x3d40, translation dropped), plus the spot +0x3d70. Not placed while the fading old motion is the toss 0x23/0x24.
- Mode 1, walk (0x21/0x22), and mode 2, toss (0x23/0x24) before release: node table entry 14 (`Bip01LFinger21`) world × (−0.05, 0.03, −0.05, 1). The node's world is built leaf upward: local × parent local × … × root, then × the player matrix (VU0 products) — top-down chaining is ≤1 µm off but only 4/970 frames bit-exact; leaf-up is 970/970.
- Mode 2 after release: the ServeData hand point × player matrix and the toss physics (already ported).
- The ball model object is player +0x13fc, its matrix at object +0xb0; the live ball (gm+0x88, +0xe0) carries the same position in recordings.
- The player matrix is a pure turn, never mirrored: left-handed Carol (character 6) serving in save state 3 has the identity matrix and her ball track lands unmirrored, bit-exact.

Verified:
- `stance_ball_ram` (tests/serve.rs): s03 (character 6, t 21) and s05 (character 0, t 7) ball model matrix translation, bit-exact.
- `anim_s05_held_ball`: anim_s05.bin (motion, sampled time) beside match_s05.bin one vsync later (rows, spot, live ball): 1316 stance frames and 970 hand frames bit-exact (a serve's first frame, before the ball is placed, and crossfading frames skipped).

App: `held_ball` system (after the motion tick) places the ball from the server's motion each tick (`hst_sim::serve::stance_ball`, `pose::node_world` + `serve::HAND_BALL`); `CharacterData` keeps the skeleton and the stance ball track (own field: the key 52 = 0x34 collides with the co05 team reaction's path in `paths`). The toon sphere is replaced by `CMN/GAME.XB` `ball1.mdl` (real size) and `ballshadow.mdl` (flat on y 0, ponytail).

Open:
- The app mirrors left-handers' figures (scale x = hand) but the game's ball is unmirrored, so in the app Carol's stance ball sits by her racket hand. Whether the original mirrors the model (and how the ball then meets the hand) needs a look at the original — PINE/screenshot hung this run.
- Pose mid-crossfade: the app poses from the motion's own clip (the game's node locals are the mixed pose).
- Shadow on stage floors that aren't flat.
