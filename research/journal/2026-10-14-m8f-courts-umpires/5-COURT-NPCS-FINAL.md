# M8f7 — court mods' people (gallery, ball kids, line judges, mascots)

## What landed

**Mods repo** (../opentennis-mods, commit "Get a Grip court galleries -> npcs.json people"):
- `tools/psp2gltf/export_getagrip_gallery.py` (run after the court exporter) adds `npcs.json`, `npcs/*.glb`,
  `npcs/*.wav` to each `out/mods/gag_cNN_*`:
  - 14 courts with a `courtNN_gallery.xb`: one billboard card glb per look `gall00..06` (the 1 × 1.95 m quad,
    one `frame_<act>` node per act texture, clips as STEP scale toggles: `idle` = act0, `groan` = act2 (head down),
    `clap` / `cheer` = act3..7 at the clap .tat's times, `*_back` = act1). People: the csv's "normal" count per
    `galleryarea.dat` box, spread at random (seeded by the court, ≥ 0.7 m apart), looks dealt round, facing the
    court centre. 11–26 people a court.
  - c09 / c16 (no gallery xb): `stands.glb`, the whole-stand crowd model once per act texture, same clips, one
    person at the origin.
  - sounds: cheer = group_glee_0..2, groan = group_sad_0..1, win/lose = group_win/lose over four voice sets
    (identical files dropped: 6 / 4 / 2 / 2), clap = the court SE bank's `clap_1` (none on c02, c16, c17).
- Standard §2b: model value may be `{"glb", "billboard": true}`; `<cue>_back` clips (camera behind the person,
  relative to `facing`) with fallback idle_back → cue; `win` / `lose` cues at match end (stand-ins cheer / groan);
  roles spelled out (ballkid `serve`/`change_ends`; linejudge `point` on out + `head` node follows the ball);
  reactions last ~2 s, staggered; sounds once from the people's middle, on top of the disc gallery.
  `notes/getagrip.md` gets a "Gallery" entry (formats above).

**HST**:
- `mods_court.rs`: `Court::people` from `npcs.json` (`people()`; bad model path refuses the file, the court
  still loads; bad people / missing sounds left out; ≤ 1024 people).
- `court_mod.rs`: `scene` can hand back its clips by name and its named nodes (`Spawned`) instead of looping
  them; `Looped::pose` shared with the gimmick loop.
- `play/npcs/court_people.rs` (plugin added from `npcs::plugin`, no play.rs change): spawns each person under
  game space (court mod's baked light), FixedUpdate `cue` (after `character::tick`, before `npcs::step`) reads
  `Game::applause` / `jingle` / phase / `flight.call` the tick the disc gallery reacts; Update `show` turns cards to
  the camera, picks front/back clips, poses them, turns a line judge's `head`. Sounds: a wav bank (program per
  cue), `play_at` the people's middle, deduped by `heard::Played::first` (rollback re-runs). No `Game` writes, no
  `g.rng`. `HST_PEOPLE_CUE=<cue>` forces a cue for shots.
- Tests: `people_cue_like_the_standard` (cue mapping, fallbacks, back clips, stagger), `reads_court_people`,
  and `a_court_mod_is_seen_and_heard_and_never_moves_the_match` now also checks the 21 people of gag_c01 are
  placed, all reacted during the two points, have sounds — with the state hash equal to the bare court every tick.
  Full `tools/check.sh`: 691 passed.

## Checks (`context/m8f7/run1/`)
- `c01_8s.png`: the far gallery stands behind the wall, upright, lit, sized like the players' umpire/kids.
- `crops2.png` (HST_PEOPLE_CUE cheer / groan through an HST_BIN wrapper — hyprctl drops env): wave/clap frames
  and heads-down frames show; the near-side people (behind the camera) pick `*_back` clips (logged).
  Left/right: the blue/white boy's blue side and the wave's raised hand sit as in the texture sheet.
- `c16_cheer.png`: the TV studio stands' seated crowd drawn around the court.
- `c09*.png`: the match camera at 8 s sits inside/behind part of gag_c09_stadium's own court.glb (same with
  `npcs.json` removed: `c09_bare.png`) — not this task's; the stands couldn't be seen from that view.

## Not verified / not 1:1
- **When and who reacts in Get a Grip**: its program is encrypted; which act frame plays at which match event,
  how many people clap vs. wave, and the cue timing are not known. Cues follow the standard (disc gallery's
  ticks), clap = the clap .tat, cheer = the same clap .tat (no other "cheer" tat exists), groan = down.
- **Back frame rule**: act1 (back view) shown when the camera is behind the person's `facing` plane — a guess at
  how GaG uses its `back` tat.
- **Placement**: GaG's area boxes are read as centre + full extents (sx, sz) with sy = height; people spread at
  random in them with the "normal" csv column — GaG's actual spots/pattern are unknown. Facing = toward the court
  centre.
- **Card light**: card normals point up, so they are lit like the ground; GaG's card lighting unknown.
- **Sounds**: crowd lines chosen from group_* cues of four voice sets; individual spectator lines and
  `clap_0` / `clap_2` / `bad_point` (unresolved SEQD group-1 cues) unused. Volumes (0x4c, 0x66 at match end) are
  ours. They play on top of the disc gallery's applause/groans (kept: the disc gallery's sounds are cued from
  the court generator).
- **Disc walkers** (the layout court's `g-MM` spectators) still walk: not hidden when a mod has people.
- **Rally threshold** `rally` from 8 shots, reaction 120 ticks, stagger 0–15 ticks: ours.
- **Ball kids, line judges, mascots**: no such people in Get a Grip's data; the roles (and the line judge's head
  turn about its parent's Y axis) are only unit-tested, not seen with a model.
- **c09 stands** not seen in a shot (court.glb blocks the match camera there, see above); c16's were.
- No comparison against the PSP game running (no PSP emulator setup here).
- Every frame node is drawn (scaled to 0 when off): ~14 draws a card person; fine at these counts.
