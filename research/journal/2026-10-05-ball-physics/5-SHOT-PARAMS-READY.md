# Shot parameter records (spin etc.) — source found; READY: needs the human's call on exe offsets

Runtime table at 0x4287c0, record = 13 f32 (0x34 bytes), index = group*0x1144 + kind*0x374 + class*0x34
(group 0..3 — not character; kind 0..4; class 0..3). Fields: +0 0.1, +4 scale(?), +0x10/+0x14/+0x18 aim/height
tuning, +0x1c spin (deg), +0x20 first-bounce spin (deg), +0x24, +0x28 (deg), +0x2c/+0x30 first-bounce
restitution pair (kinds 3/4). Captured spins match exactly: 170°, 100°, 417.5°, 227.5°, -120°, -90°, 293.75°.

Built by 0x37be10 at init: classes 0..2 copied from static data in GAME.BIN (file 0xe1b10 = 0x404810),
class 3 = blend between rows with per-kind weights (0x403f60.., counts per group 0x403b90). e.g. 170 = mid(240,100).

DECISION NEEDED: the table only exists inside the executable overlay. Options:
 a) hst-data reads it from the user's GAME.BIN at runtime via a documented, disc-ID-checked offset
    (exception to AGENT.md "no retail offsets": data extraction only, never code addresses);
 b) ship our own tuning values (breaks "faithful" unless copied = retail data in git — not allowed).
Recommendation: (a).
