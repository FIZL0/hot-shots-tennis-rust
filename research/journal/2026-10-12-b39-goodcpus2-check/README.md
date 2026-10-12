# B39 — checking the remaster against `1p3goodcpus2.p2m2` — FINAL

Capture: `context/fixtures/1p3goodcpus2.bin` (+ `.cam`, `.pad`, `.setup.json`). It is the user's recording replayed
with `tools/play_p2m2.py` in four segments and stitched together by slot s1. The seams were dropped at vsync 9300, 11000
and 12700, and there is a gap at 12365–12400. It is a doubles match on court 4 with one human (P1) and three CPUs.

## (a) Lobs: they match
`crates/hst-sim/tests/live.rs` `goodcpus2_lobs_and_fence_hits_match_the_game` replays every flying frame of the live
ball against court 4's collision world. Every frame is bit-exact (>2000 frames), including the five lobs (shot kind 3,
>100 frames in flight). Nothing to fix.

## (b) Changing sides
This was moved to B49.

## (c) The ball hitting an object in the background
- **Physics match.** The same test covers six contacts with material 14, the back fences behind either baseline
  (vsync 8052, 9168, 10148, 11214, 11971, 13323). Each one comes right after the point is decided (the points end at
  8048, 9170, 10147, 11188, 11966 and 13314). The bounces are bit-exact. Material 14 has restitution 0.4, effect kind
  9 and sound key 11.
- **The original shows a word pop-up the remaster lacked: RATTLE** (`panel/cf00-metal02.tm2`).
  - The rally effects object (its update, draw and message handler) makes a word pop-up for each new ball contact
    with a material whose effect kind is not 0 and not 5. Its texture index is 12 + kind, and kinds 1..15 are
    `cf00-` + netpole, netup, netdown, yuka, kusa, suna, tuchi, metal01, metal02, metal03, cloth, wood, water, taki
    and tamari.
  - The gating is the same as the material sounds' (`sound::Bounces`): the contact count must be below 6, the 4-frame
    cooldown must have run out, and the soft net's word can't repeat before a court contact. There is also a limit
    of 10 words per point.
  - The flag that allows spawning is set when the rally phase starts (at the serve's hit) and is cleared only by the
    next point's reset. So, unlike the sound, the words still appear after the point is decided. That is why every
    fence hit in this capture shows RATTLE.
  - A word is placed at the contact point. Its balloon-table row is size 0.3, near 0.18 and far 0.3, with no fade-in,
    a 20-frame hold and a 5-frame fade-out. It doesn't rise or swell. Its anchor is the same as the body-hit words'
    (0.25 m up, 0.5 m toward the camera).
  - Alpha is 128 for 21 frames (it ticks on the frame it is made), then 102, 76, 51, 25 and 0.
- **Ported.**
  - `hst_sim::bodyhit::{BounceWords, BouncePop, bounce_texture}`, with the quad shared with the body-hit `Popup`.
  - The test `bounce_words` checks the alpha sequence, the cooldown and the net rule.
  - `crates/hst/src/play/bodyhit.rs`: the `bounce_words` system feeds the words from the serve's hit until the next
    point's `Phase::Serve`, and `draw_words` draws them as up to 10 camera-facing quads. play.rs is untouched.

## (d) The NPCs' clap pop-ups
These were already done by P3e6 (`play/npcs.rs` `draw_cheers`), and their height was checked on court 4. Nothing to
change.

## (e) The post-point camera
- Layout of the `.cam` track, inferred from the data:
  - byte 6 is 1 during the post-point phase;
  - 0x20 is the pending cut-away mode (0xff = none);
  - 0x21 is the mode;
  - 0x24 is an int (±1);
  - 0x2c/0x2d are the last shots.
- The original's 7 cut-aways in the capture are all modes that `cutaway::pick` can choose for those points. No
  instant replay follows any point the human team won, which answers B48's open question for this recording.

## Not verified / not 1:1
- **Bounce words, by eye:** I didn't take side-by-side screenshots of RATTLE (PCSX2 near vsync 8052 vs `--shot`). The
  remaster can't replay a p2m2, and no bot match lands a fence hit at a known frame. The size, timing, texture and
  anchor come from the decompiled code, and the draw path is the same as the body-hit words, which were checked by
  screenshot.
- **The rolling-ball word (kusa) is not ported.** The original also pops one up for a ball rolling over a material at
  ≥ 0.08 m/frame. That uses a flight flag the port doesn't model, and no capture shows one. It is marked `ponytail:`
  in `BounceWords`.
- **The 10-words-per-point limit also gates the material sounds** in the original. `sound::Bounces` doesn't apply
  it. This only matters after 10 obstacle contacts in one point, which never happens in the captures.
- **The position of the word** comes from the port's last contact point (`Flight::landing`). The original uses its
  contact record n−1. They should be the same point, but I didn't compare them.
- **(e) camera:** the team-vs-single inputs of the cut-away picker can't be checked frame by frame, because the app
  can't replay a p2m2. Only the modes chosen were compared.
