# B506: creatures off the court (court mods)

**What it was.** "University (green)" is Get a Grip's c14 court mod (`gag_c14_academy`). Its `gimmick.glb` places six
pigeons (`gi08_HATO_r20..25`) where Get a Grip's csv puts them, looping their resting `Stay` motion; one (r24) stands
inside the singles court at (2.7, 5.3) m, three more in the alleys/run-off. In Get a Grip they fly off along their
route when play comes near (the encrypted PSP program's reaction, never reproduced, M8f5), so in HST they stayed
in the rally for the whole match.

**Fix (loader, `hst-base/src/court_mod.rs`).** `setup` spawns `gimmick.glb` through `scene_off(.., off_court)`;
`creatures_in_play` lists the mesh nodes left out: every skinned mesh whose box at rest enters the players' room
(`in_play`: doubles court + 2 m beside / 4 m behind, as B477's walker room; 3 m up to 1.5 m down for mod grounds
off the layout's) and the rigid meshes that go with it (climbing from its skeleton while no other skin and no mesh
out of the room comes in: the shadow disc, the Doberman's collar). Get a Grip's creatures are all skinned and its
props rigid, so baskets, ball carts, panels, poles stay. Draw only: nothing of the match reads it (replays stay
bit-exact). `scene`'s signature is unchanged (court_people's npcs.json people aren't filtered: the author places them).

**Every court** (`no_court_mod_creature_stands_in_the_rally`, `HST_MODS=mods/hst-mods`, all 16 court mods, singles
and doubles; each left-out mesh small, on the ground, with its shadow):
c01 4 pigeons · c03 sparrow · c04 crab · c07 pigeon · c08 Doberman · c09 **6 pigeons on the court** (a flock over the
near half) · c14 4 pigeons · c18 tiger · c20 3 crows. c02, c05, c06, c13, c15, c16, c17: none.

Stock courts (scan of `npc::spawn` over courts 0–11 against the same room, throwaway test, not kept): walkers and
umpire stand 6–7.5 m from the centre line, beside the court, as the original places them; trigger creatures' paths
come into the room only at court 2 (type 5's path node at (5.8, -1.7), 0.3 m outside the doubles sideline), court 6
(type 22 at x -6.5), court 8 (34 at x 7.3, raised), court 11 (48 at x ±7.3) and court 5's type 19 at the centre
(y 0, radius 1). These are the original's data and paths, ported bit-exact against RAM (P14): left as they are.

**Seen** (`context/b506/run1/`, `HST_WEATHER=0`, main's binary as before): `ba.jpg` (c14, held eye over the court
at the pigeon) the pigeon in the court and one in the alley before, gone after, the rest identical; `c09.jpg` (match
view) the six pigeons on the near half before, a clear court after.

## Not verified / not 1:1
- Not compared with Get a Grip (no PSP capture): there the pigeons are seen until they fly off; here the ones in the
  players' room are never drawn. A choice for HST (court mods are additions), not a port of their reaction.
- Rigid props in the room are kept as Get a Grip placed them (c01/c02 ball baskets behind the baselines, c14/c16/c20
  ball carts, c16's camera boom and panels, c18's firecracker poles reaching into the doubles alleys, c04's penjor
  poles): props, not creatures; a player can run through them.
- c06's butterflies are rigid, node-animated and fly beside the sidelines: not skinned, so kept.
- "Creature = skinned" holds for Get a Grip's export; another court mod with a skinned prop in the room would lose it,
  and a rigid creature would stay.
- Box at rest only: a creature whose loop carries it into the room from outside isn't caught (none of Get a Grip's
  rest loops move their root).
- c01, c03, c04, c07, c08, c14 (match view), c18, c20 not shot; covered by the test's numbers.
