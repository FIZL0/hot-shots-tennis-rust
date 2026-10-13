# B144 — Ashley's mouth open in the PNG, closed live

**Which is right.** The open mouth. Ashley's inspect pose holds frame 69 (`character::pose_frame`). Her pose face is
70 frames long, so the clock clamps to 69. Her `.MOR` tracks give `joy_mouth` 1.0 there (0.15 at frame 0) and
`joy_eye` 0.06, so she smiles with her mouth open. A temporary test printed every track at frames 0/60/68/69/70.

**Cause.** This is B167's bug (landed after B144 was filed). On native/WebGPU, a preview spawned Hidden until posed
kept a mesh uniform with no morph descriptor, so its face drew neutral (mouth closed) on any rig that went through the
hide. That gave the disagreement the user saw. Their cache `cache/previews/SCUS_976.10-r6/d00-0.png` (baked before
B167) shows the closed, neutral mouth. B167 fixed the reveal and bumped RENDER to 7, so the cards re-bake.

**Check now** (captures in `context/b144/`). I ran `HST_MENU=chars HST_SELECT=0,0,0,0`, using the disc and the user's
mods folder, with `HST_PREVIEW=live` and `HST_PREVIEW=png`:
- Both have the mouth open.
- At the 1:1 card size (`HST_SIZE='1494 840'`), the face crop differs by a mean of 1.2/255 per channel (`face1.png`).
- At 1280×720 the PNG is resampled, so eyes and teeth shift a few pixels (`mouth.png`). Same face, as B140 noted.
- The fresh r7 bake (`d00-0.png`) is open; the user's old r6 bake (`user-head.png`, left) is closed.

**Change.** No fix was needed. I added a regression check to `inspect_poses_bind_and_hold_the_games_frames`: Ashley's
held face clock samples `joy_mouth` = 1.0.

## Not verified / not 1:1
- I didn't compare against the original's inspect screen in PCSX2 (the select is ours to design, and rendering only
  has to look right). The open mouth follows the disc's face data at the frame the original holds.
- The user's own r6 cache only re-bakes once they run a build with RENDER 7 (automatic, as the folder name changes).
