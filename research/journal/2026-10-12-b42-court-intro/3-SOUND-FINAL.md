# B42c — court intro music and sounds (FINAL)

## Original
- `research/b42c_sound_rec.py <out> [skip frame]` (under `tools/pcsx2.sh`): from save slot 2 into a doubles match,
  records every vsync the 8 SMF player slots and the sound driver's command ring beside the match frame, phase and
  mgr+0xd94. `B42C_MUSIC=1` keeps the match music option on (slot 2's save has it off). Runs in `context/b42c/`:
  run1 court 8 (no skip, music off), run2 court 4 (✕ at ~101), run4 court 5 (music on).
- What phase 0 waits on (0x167310) is not a streamed sound: it is the screen transition object (a fade with a
  frame count, 1/n step) — "is a fade still running". The B42 plan line is corrected.
- Message 6 (entering phase 0, 0x323990) → director 0x1a11a0: with the music option (0x4230c6) on, the court BGM
  starts at volume 55 (the bank was loaded by the match setup, 0x322d90 → 0x19a920, track court + 7 = `bgmg_NN`).
  Recorded (run4): SMF slot 5 starts during the loading screen while the match frame is still 0, and is never
  restarted at the intro's end or the first serve. With the option off (run1/run2) no SMF slot runs at all.
- The gallery cheers through the whole intro: program 10 from stand to stand, scale words 0xe2a..0x13cf, each
  re-keyed 18 ticks later (the cheer sequence), first at frame 1, 12–40 ticks apart; no new cheer after phase 0 ends.
  This is `sound::Gallery` started with no event (`0x39da50` with the cheer flag).
- Message 10 (flyover end + 2 or ✕) starts a 16-frame fade (0x382500; counter = 16 + 2) and arms 0x38a3d0's
  flag; 0x38a410 plays slot 9 (system bank `sys_se00`) program 0 key 8 at 0x80 on the first tick the fade is
  done: frame 319 on court 8 (banner at 302), 118 on court 4 after the ✕. Identified through
  `research/tools/fixture_spu.py` on a scratch save (SPU 0x1f4700 = `sys_se00` + 0x14700 = program 0 key 8's
  tone, notes 0x29/0x2a as in the ring's pitch words). fixture_spu.py no longer dies on a ring without an upload.

## Port
- `play/intro.rs` `sounds`: at the intro's start the court BGM (with `--music`, `MusicBanks::bgm`) at 55; from
  tick 1 a `Gallery` cheer on its own copy of the court generator (the match's generators stay as the setup left
  them); `sys_se00` 0/8 at slot 9 on banner + 17 (`BANNER_SOUND`; the banner tick is the flyover's end + 2 or the
  ✕'s tick). No cheer in the rain, as in the match.
- `audio.rs` `Sound::music_of`; `play.rs` `Bgm::step` (one line) doesn't restart a BGM the intro already started.
- Check: a court 8 run (`HST_INTRO=1 … --stage 8 --play --music`) logged the BGM at tick 0, the cheers at
  1, 21, 38, 67… (stand order 180, 45, 225, 90…), the banner sound at 319 — the original's 319.
  `plays_music_and_jingles` checks `music_of`.

## Not verified / not 1:1
- The loading-screen delay: the original starts the BGM while its loading screen holds frame 0 (~60 vsyncs into
  phase 0); the remaster has no loading screen, so the BGM starts with the flyover's first tick.
- The cheer's random draws (stand, pitch, interval) use a copy of the court generator; in the original they come
  out of the shared court MT, so the remaster's draws differ from the game's after the intro (presentation only;
  the original's court generator also runs the walkers during the intro, which the port doesn't).
- The cheer's last ticks: whether the gallery is hushed at phase 2 or simply has no cheer due in the last
  ~12 ticks wasn't separable from two recordings; ours stops with the intro.
- The 16-frame fade of the cut to the match camera itself (visual) isn't drawn — B42b's banner side.
- Singles intro sounds (1 player loads track 0x13 instead) and courts other than 4, 5, 8: not recorded.
- Sound levels are by ear (same `play_centre`/`play_toward` paths as the menu and the match).
