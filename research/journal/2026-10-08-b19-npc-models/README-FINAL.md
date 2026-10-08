# B19: umpire, walkers and trigger creatures drawn with their own models

## Where the files are
Everything is in the court's `COURTSET/NN/HOLE01.XB` (extracted copy: `context/xb/COURTSET/10/HOLE01.XB/`):
- Umpire: `data/azuma/Umpire/uXX/uXX.MDL/.MTL/.MTI` with `uXX_h/_r/_l.ANI` (motion 0 idle, 1/2 turned). The app
  uses `u04` (Lily). `.TAF/.TAI` hold a texture animation, which isn't ported.
- Walkers: `gallery/mdl/g-MM.*` with `gallery/ani/npcSS/npcSS_01..07.ANI` (anims 0..6); MM/SS come from
  `exe::Game::walkers`.
- Trigger creatures: `trgCreMdl/<model>.MDL` with `<base>.ANI`, or `<base>_a/_b.ANI` for two clips, from the
  game's creature model table (`exe::Game::trigger_model`, 0x18-byte rows: name pointers +0/+4, has-anim +8, clip
  count +9; MOR/MTA flags at +0xa/+0xc aren't used). Types without a model have a null name pointer.
- All of them are skinned ANI2 models (80 ticks per frame), so `character::load_npc` reuses the player pipeline
  (`skinned_parts`, `Clip`).
- `hst-data/tests/npcs.rs`: on all 11 courts, every figure the roster makes has its model and clips, which parse
  (94 models, 470 clips).

## Materials
The gallery models end in a 4-vertex ground-shadow quad: material TEST mode 25, colour alpha 0.6, and a 32×32
soft blob in the texture alpha. Drawn opaque (as the players' materials are), it showed as a black rectangle.
NPC materials now follow the mode: 10..19 alpha-masked at 0.5 (A ≥ 0x40), 20..29 blended.

## Motion (crates/hst/src/play/npcs.rs)
- Umpire: her matrix faces the court centre (rows up×fwd, up, fwd, chair). On a motion change she restarts at
  frame 0. She steps one frame per tick while turned, or once the match is over; idle stays on frame 0 during play.
  A turn that reaches its end goes back to frame 16 (14 for umpire 3, not used) and plays on from there.
- Walkers: `npc::Walker` per walker, with clip lengths from the clips. On a decided point they `react()` and
  `npc::cheerers` picks who cheers: all of them below 4 walkers, else 3 distinct draws `(r>>16 & 0x7fff) % n`,
  redrawn on a repeat. At the next serve `new_point(doubles)` runs and the gallery tick restarts.
- The app sees both events from game state, without touching play.rs's point code: a decided point is the
  umpire's motion going 0→1/2 (`point_over` sets it only then), a new point the next `Phase::Serve`. Both land one
  tick after the original.
- Trigger creatures: `npc::Trigger` with no path, reset at each new point. Types 34/44 animate through their
  callbacks; the others hold frame 0.
- The draws come from the app's rng (`Game::rng`), as the emitters' do.

## Compared
`context/b19/orig_s05.png` (PCSX2 slot 5, court 10) against `port_9.png` (`--play --stage 10 --shot-at 9`):
- The umpire sits on the right-hand chair facing the court, as in the original.
- The left walker by the channel and the pink one at the top right stand where the original's do.
- The original's left walker faces the court; ours faces the far end. The npc test shows walker matrices are
  bit-exact wherever a walker hasn't moved, so on court 10 the original's walker has walked or turned (P14e, not
  ported).
- Log over 20 s of autoplay: the idle loops step at 2 frames every other tick with four players. After a point,
  three walkers cheer (anim 5) and one reacts quietly (anim 2). The umpire turns (motion 2) and returns to idle at
  the serve.

## Not done (ponytail)
- Umpire head look-at toward `Umpire::head`, and the texture animations (TAF).
- Walker movement and facing the winner (P14e).
- Creature paths: moving creatures stand at home. Two-clip creatures play clip `_a` only. Creature sounds.
- Court 5's own creatures (kinds 61+) aren't drawn; their models haven't been found.
- In singles, `new_point` doesn't stagger, so a walker that cheered (anim 5 loops) keeps cheering. This follows the
  decompiled rule but is unrecorded.
- The court viewer without `--play` no longer draws the capsule stand-ins; the figures are play-mode only.
- Mode 20..29's Z-writing A ≥ 0x70 half.
