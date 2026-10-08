# P26: match stats and the stats screen

Ported: the counters in `crates/hst-sim/src/stats.rs` and the screen in `crates/hst/src/play/match_stats.rs`.
The screen opens at match over and ✕ continues. `HST_STATS=<s>` forces it open for `--shot`.

## Where things are in the original

- **Stats object:** a pointer at 0x3165c0, class functions 0x19e360..0x19f4f0. Per-player blocks start at +0x54 with stride 0x94.
- **Events:** the event handler is 0x19e4e0.
  - 0xe is serve start.
  - 0x17 is point end, which runs 0x19f030. On a let it undoes the serve count instead.
  - 7 and 0x1a switch the time counter on and off.
  - 6 clears the counters.
  - It snapshots at +0x2a4 and restores while the instant replay is playing (flag 0x422f80+0x344).
- **Per frame (0x19eae0):**
  - Serve speed: km/h (`sound::kmh`) × 0.9, taken the first frame of shot 1.
  - Smash top speed into +0x74.
  - On each new shot number:
    - hits +0xa8;
    - shots ≥3 +0xbc, and of those within |z| ≤ 6.4 of the net +0xb8;
    - serve struck +0x5c;
    - branch +0x3ec1: 4 (smash) counts +0x6c, 3 (dive) counts +0xc0;
    - the exchange counter +0x564 goes up on a hit after the other team's hit (it counts the receiver's team's hits, from the third shot on).
  - Distance run goes into +0xb0 (not shown).
  - Match time +0x4f8 (frames, capped at 21599940).
- **Sweet spot:** +0xa4 counts in the balloon code for the sweet balloon (grade 1 or 2, |offset| < 2, not a dive).
- **Forehand:** the player's +0x17d0 counts at the swing (0x346f..) on branches 1–2 when the motion's flag bit 0 is clear (not during replay). The port uses `!backhand`.
- **Scoreboard verdict (0x38c550):**
  - Out (call 1) counts +0x84 for the hitter; out after the net (call 5) counts +0x88. In both cases the point goes to 0x316600.
  - A point goes to its hitter.
  - An illegal hit gives the point to 0x316600.
  - A double fault gives nobody a point.
  - Ace (+0x54): the serve was the only shot (0x3165f4 == 1), call 0, and the body-hit player 0x42305c isn't on the server's team (0x38d6f0).
  - +0x64 and +0x70 count other winners (not shown).
- **Point end (0x19f030):**
  - Fastest serve +0x98 = max, unless the call is 2, 3 or 4.
  - Double faults +0x80 when faults == 2.
  - Longest rally +0x4f4 = max (≤ 999).
  - Rates:
    - sweet% = a4/a8;
    - net% = b8/bc;
    - forehand% = forehands / (hits − serves − dives − smashes).
- **Game won (0x38b820):**
  - +0x57c counts games.
  - Each team's share is `round(100/max(p0,p1)·p)` of the game's final points, summed into players 0/1 at +0xe0.
- **Match end (0x3ac090):**
  - rating0 = round(avg0/(avg0+avg1)·100).
  - 50 becomes 51 or 49 for the winner.
  - The winner never rates under 51.
  - The 9 display rows (stride 0x24 at obj+0x2194):
    - mph = (int)clamp(kmh) × 0.62137;
    - ints clamped to 999;
    - rating 0..100, players 0/1 only.
  - Banner index (narrow, win, domination) by the winner's rating 50–64, 65–79 and 80+: not used.
- **Screen draw:** 0x3ad2a0 (layout decoded in the code). The header faces and slot labels are 0x3a7540. The winner pill's alpha pulses 0↔102 over 21 ticks (0x3aec..).
- **Textures:** `AZUMA/PRIZE/OTHER.XB0`, `result_status00` (frames/stripes) and `result_status01` (digits, labels). Paths are `mtl/prize/...`.

## Left out

- **The match-over result page:** banners (`result_gameset05`), "Stats Screen"/"✕ Continue" and the page slide.
- **Row-name gradient:** the names use one tint.
- **Background sprite:** obj+0x84, a base-class sprite, is drawn as white.
- **Character names under the faces:** texture +0xd44 of the HUD object.
- **Point credit:** points won after an out go to the other team's last hitter, standing in for 0x316600.
- **Verification:** not checked against PCSX2 RAM. The bot match needs a whole match to reach the screen.
