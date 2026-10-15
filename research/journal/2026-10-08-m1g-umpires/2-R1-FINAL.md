# M1g umpires as playable characters, R1 (FINAL)

This replaces the first pass in README.md. That pass shipped `test_pc00` as a placeholder model for every umpire.
Those `mods/umpire_0..4` placeholders are deleted (they were git-ignored, removed with `rm -r` and no glob).

## Pipeline
Run `python3 tools/rerig/hst_umpires.py [open-tennis dir]` in open-tennis-mods (commit bc37b03). Its output goes to the
git-ignored `out/`. In the game the mods show up as `mods/hst-mods/hst_umpire_NN_<slug>`.

1. `hst-gltf umpires context/xb out/models/hst/umpires` exports `u00..u04.glb` from the umpires' own MDLs, with
   their `_h/_r/_l` chair clips. This is a new subcommand in `crates/hst-gltf/src/main.rs`.
2. **Stand-up.** The bind pose is seated, and the leg vertices are skinned 100 % to the pelvis. The script does the
   following:
   - Reweights the pelvis-only vertices to the nearest two segments, with a softmin over 2 cm. The segments are
     pelvis→spine and, on each side, Thigh→Calf→Foot→Toe0→end.
   - Swings Thigh→Calf and Calf→Foot to the bind directions in `hst_skeleton.json`.
   - Pitches each sole level about the ankle, using the lowest heel and toe points.
   - Puts the floor at y = 0 and rewrites the IBMs.
3. `rerig.rerig` maps the result to the 54-joint standard. `check()` must pass.
4. **Donor and stats.** Umpires have no stats, AI row or handedness in the game, so the mod takes the donor's TParam
   row and AI row and uses the right hand. The donor is chosen as follows:
   - Suzuki (u01) → Suzuki (13). He is the same Suzuki as playable pc13.
   - Chika, Lily and Anna → Carol (6). The rule is the nearest Bip01 height among same-sex standard bodies
     (model_type 0/4/5).
   - RobotTennis → JJ (5). The robot has no sex, so it takes the nearest Bip01 height over all 14 characters.

   | Umpire | Bip01 | Donor |
   |---|---|---|
   | Chika | 0.743 m | Carol |
   | Suzuki | 0.721 m | Suzuki |
   | Lily | 0.783 m | Carol |
   | RobotTennis | 0.431 m | JJ |
   | Anna | 0.728 m | Carol |

5. **Voices.** A new tool, `crates/hst-data/src/bin/bankwav.rs`, dumps each one-note take of `SND/UMP/UVnnA.XB0`
   as `<p>_<k>.wav`, at the rate the game plays it. The umpire's own calls are used where their meaning fits a
   player reaction:
   - 7 point won → "Game" (2_1)
   - 8 point lost → "Out" (1_0, 1_4)
   - 9 set won → "Set" / "Match" (2_2, 2_4)
   - 10 set lost → "Double fault" (1_2)
   - Every other program plays from the donor's disc bank.
6. **Donor fallback, engine side.** Before this change a mod with any wav was silent on the programs it didn't ship.
   - `play/mod_match.rs::apply` now copies the donor's disc `hd/bd` into the mod's bank.
   - `audio.rs::Sound::play` plays a mod wav when that program has one, else the hd path.
   - Standard §5 in open-tennis-mods documents the rule.

## Roster
B40a's custom roster lists `mods/` (`mods::list`). The five appear there, after the disc cast, in id order.

## Tests
- `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally` checks the following for each mod:
  - all five ids are listed in order;
  - tparam is OK;
  - `mods::load` gives 54 joints;
  - a Carol-vs-mod bot match reaches a rally of 3 or more shots with the mod hitting. Results:
    chika at frame 301, suzuki at 294, lily at 301, robottennis at 323, anna at 301.

  The test skips when there is no ISO or no `hst_umpire_*` mods.
- `audio::tests::plays_mod_wavs` is extended so that a program missing from the mod plays from the donor's bank.
- `tools/check.sh -p hst`: passed, 349 tests, 0 failed. The script printed "0 tests" because the log holds binary
  bytes and grep miscounts them; the real count comes from `grep -a`.

## Screenshots (context/shots/m1g/)
- `chika.png`: Chika (slot 1, far side) in a singles bot match at 8 s.
- `robot_play.png`: RobotTennis (near side, 1P) mid-swing on court 1 at 12 s.

Both shots show the players standing, with HUD and NPCs. The court surface isn't drawn in either shot. The
same thing happens with no mods at all (`disc_only.png`, JJ vs Carol), so the mods don't cause it. It is a
`--shot` or worktree issue and is outside this task. `roster.png`
(`--inspect` with the mod ids) came out blank pink, probably because it takes mod dirs and not ids. It was not
pursued.

`tools/shot.sh` failed until `HYPRLAND_INSTANCE_SIGNATURE` was pointed at the live instance
(`…_1791785634_…`). The shell inherited the signature of a dead earlier instance.

## Not verified / not 1:1
- **The stand-up pose is made up.** The game never shows umpires standing. The legs are re-skinned by proximity, so
  the bends at hip and knee are approximate.
  - Anna's feet are not levelled: too few points sit below her ankle, because her leg joints are behind and below
    her shoes.
  - Chika's left shoe is slightly toe-down, and her feet overlap a little.
  - Lily is slightly knock-kneed, and the robot's feet are slightly tilted.
- **The robot's Bip01 is 0.431 m**, which is a check.py WARN (below 0.4–1.3 m is WARN, not FAIL). It is the model's
  real size; nothing was scaled.
- **No face morphs** (`face: "none"`). Umpire MDLs have no player face set.
- **The motions are the donor's.** The umpires' own chair clips are exported but not used.
- **The voice mapping goes by meaning and was not checked by ear.** Only bank A (`UVnnA`) is used. Multi-note takes
  such as 0_13 are skipped.
- **Stats, AI row and hand.** The stats and AI row are the donor's and the hand is right, because the game has no
  such data for umpires.
- **The donor fallback now applies to every mod.** For example, a Get a Grip mod with no program 2 now plays the
  donor's mis-hit instead of nothing. This is intended, but it changes what those mods sound like.
- `bot_match_with_a_mod_plays_a_rally` is still skipped, as it was before this task: `context/mods/test_pc00` is
  missing.

## Done when
Every umpire is in the select, can be picked, and plays a rally. Met: the test passes for all five.
