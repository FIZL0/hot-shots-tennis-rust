# M108: Android app in the background (2026-10-15)

## Problem
Leaving the app (home, recents, screen off) left the music playing. Bevy's winit runner already stops updating
once the surface goes (`AppLifecycle::Suspended`), and the patched winit draws nothing while the activity is paused.
But the audio is a cpal/rodio stream on its own thread that keeps pulling from the mixer (`audio.rs`), so the music
and any held loops kept sounding.

## Change
- **winit fork** (`third_party/winit`): new `platform::android::lifecycle` with `set_hook(fn(Phase))`. The event loop
  calls the hook for android-activity's Start / Resume / Pause / Stop / Destroy (upstream only logs "TODO: forward …").
  It runs on the event loop's thread even while Bevy's updates have stopped.
- **`android.rs`** `lifecycle` hook (installed in `main.rs`):
  - Pause/Stop: `audio::set_muted(true)`. The decoder outputs silence and doesn't call `Mix::render`, so music and
    sounds stay where they were and carry on from there on Resume.
  - Online match: a thread waits `GRACE` (1 s). If the app is still paused after the same pause, it sends
    `Cmd::Leave` through `Net::leaver()` (a clone of the command channel). That is the same leave as quitting the
    lobby or match, so the net thread closes the connection with "left".
  - Destroy with `isFinishing()` true: `process::exit(0)`. Destroy for a configuration change (seen on the emulator
    at first launch, `CONFIG_ASSETS_PATHS`) must not exit; my first version exited on every Destroy, which killed
    the app at launch.
- **`android::background`** (base app, `First`, so it runs before the match's fixed ticks): it keeps the leave handle
  for the online match that's on (`play::background::leaver`). On the first frame after a pause it calls
  `play::background::back`:
  - Online, left: the match ends ("You left the match (the app went to the background).") and closes back to the
    menu after the usual end notice.
  - Online, back within the grace: nothing happens.
  - Offline: the pause screen opens (not on the result screen), so the rally doesn't carry on before the player
    looks.
- The game loop needed no change: it already stops while paused (see Verified).

## Verified
Emulator only: hst34, API 34, x86_64, own `-read-only -no-window -port 5654` instance, APK from
`HST_VERSION= HST_ABIS=x86_64 tools/apk.sh`, disc.pack pushed. Results from `hst-log.txt`:
- **Home**: `activity Pause` → `lifecycle Suspended` → `activity Stop`. No frames while away ("23222 ms to the next
  frame"). Brought back: `Start`, `Resume`, `lifecycle Running`, and frames go on (the window was made again with no
  crash).
- **Screen off/on** (KEYCODE_SLEEP/WAKEUP): the same sequence, about 12.7 s with no frames, and it resumed fine.
- **Removed from recents** (`cmd activity stack remove`, what the swipe does): the process is gone. The system killed it
  itself (no Destroy reached the hook).
- Opening recents alone doesn't pause the app on Android 14 (it stays resumed as the live tile and keeps drawing).
  That's how Android behaves, not a bug.
- Unit test `play::background::tests::an_online_match_left_in_the_background_ends`: a quick return keeps the match
  going, the leaver sends `Cmd::Leave`, and returning after a leave ends the match.

## Not verified / not 1:1
- **Real phone**: an SM-S931U was on adb at the start, but it disconnected before the APK finished building, so none
  of this ran on a phone. → plan/HUMAN.md-style check: home, recents swipe and screen off on the phone; listen that
  the music stops and comes back.
- **Audio silence** isn't heard on the emulator (`-no-audio`). The mute is a flag the stream's decoder reads, so the
  cpal/AAudio stream itself keeps running and outputs zeros: a little battery use, and Android still sees an active
  player.
- **Online grace/leave** was only tested by the unit test. No two-peer match on a device (SwiftShader is too slow for
  a match). During the 1 s grace the game loop is stopped too: the connection stays up but no pads or keep-alives
  go out, so the peers see a short stall ("Waiting for 2P…"). That's well under `recover::SILENT` (3 s), so they
  don't mark it away.
- **Offline pause screen on return** wasn't seen on screen (no match was started on the emulator). The code path is
  `menu::set_paused`, which peers' pauses already use.
- Front-end/presentation only: no gameplay change. The sim is untouched; the match just doesn't tick while paused.
- The online *lobby* (not a match) isn't left on backgrounding. The task only covers matches.
- Not a gameplay/1:1 matter: the original has no background state.
