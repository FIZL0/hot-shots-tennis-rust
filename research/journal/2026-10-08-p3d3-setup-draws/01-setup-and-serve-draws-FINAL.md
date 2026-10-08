# P3d3 setup and singles serve draws

## Setup order (0x322d90)

Confirmed offline on 7 save states (copy 3 slots 05/03/07, context/recordings bots_singles, 1p3goodcpus2, new_recording, serve_recording) via research/p3d3_setup_ram.py -> context/p3d3/setup_ram.bin.

- Weather schedule.
- Per player, in the constructor 0x344740: voice-bank pick (0x345ad0, r15%100 >= 70 -> b); AI reseed (0x35edb0) if it has an AI object; a placement draw (0x3449f0) that the app was missing (P3d's journal didn't list it).
- Then the 0x33d920 constructor's hit-spark roll 0x341fb0 -> 0x343140 -> 0x343200 mode 1 (25x4 = 100 draws).
- Then the gallery pick 0x19d270(mgr, -1): uniform r15 % count over the unused flags (mgr+0x1d0..0x1d3), marks it used, stored at 0x3125a8; all used -> reset and recurse. The no-repeat check never fires at setup (param -1).

The "BGM variant" in the task text is really the gallery bank: `snd/COURT/c_snd%02d%c.xb` with 'a' (pick < 2) or 'b'; slot 6 gets (pick&1)<<4|court.

Without the placement draw no state matches; with it all 7 match voice flags, AI generator (reached in 89-184 draws) and pick. Seeds found by walking rand() back: slot5 0x28c7c4a1, slot3 0x2b749cd5, court-1 0x35a5f444, bots_singles 0x5812baaf, 1p3goodcpus2 0x71915528, serve_recording 0x6d896b4a.

## Singles new point (message 0xe, 0x348a00)

Per player, in object order: placement draw (always); then if players == 2 and the player is the server (+0x3fa4 == 1) and (0x423040 or (+0x3b94 and r15%100 < 50)) -> 3553d0(p,6,0,1,2): one key draw over keys 0..1 excluding memory slot 2's last key, plays program 6.

- 0x423040: no serve struck yet this match; set on message 7 / rematch 0x1b, cleared on 0x12.
- +0x3b94: set on change ends 0xc, cleared on point-over exit 0x18.

Voice only; no animation found in the decompile (the task text said "animation and voice").

## Live capture (research/p3e5_draw_log.py, now with HST_V0 for the load vsync window)

Slot 8 = bots_singles court 1, then slot 9 saved at deuce of game 1.
- First 0xe at vsync 5304: placement p0 (ra 0x344a14), p0's key draw (ra 0x35546c, caller 0x348ebc), placement p1.
- Change of ends at 15052 (0xc: two placements), then 0xe at 15133: placement p0, placement p1, p1's (server) roll (ra 0x348e84), no key (roll >= 50).
- No other point shows extra draws.
- Files: context/p3d3/shared_singles.bin, shared_ends.bin.

## Port

- `Rngs::setup_gallery` (hst-sim rng.rs): 100 spark draws + gallery pick.
- play.rs setup draws placement after each player's AI reseed, then setup_gallery, and loads the gallery bank (`audio::gallery_bank`, replacing audio.rs's clock pick).
- `placement_draws(g, ends)` adds the singles server voice with a `first_serve` Game flag (set at setup and on match over); `ends` = after ChangeEnds.
- Test `setup_draws_like_the_game` (crates/hst-sim/tests/rng.rs) asserts voice flags, AI generator and gallery pick on all states (fails if the placement draw is dropped). The serve-voice draws are checked against the capture's order only (the log has no drawn values).

## Gaps

- Gallery used-flags are kept across matches in the game (app runs one match per process): P3d5.
- Doubles message 0x10 voice (0x348a00: players == 4, +0x3b96 unset, a hud check 0x38afd0, `3553d0(p,6,2,2,-1)` then +0x3b96 = 1) not ported: P3d4.
