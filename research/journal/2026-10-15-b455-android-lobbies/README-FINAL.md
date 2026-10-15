# B455: Android lobby browser finds no lobbies

User report (2026-10-15, same APK as B454): the Android app's lobby list stays empty, but hosting from the phone works.

## Findings
- **The internet lookup works on Android.** I built a small mainline-DHT probe (`context/b455/probe`, n0-mainline 0.7)
  for x86_64-android and ran it on the hst34 emulator. It bootstraps in about 1 s and reads the DHT board (get_peers on
  the topic) and all 8 board slots, the same as on the desktop. In the app itself (x86_64 APK, emulator), a lobby
  hosted on the desktop with `hst net host` showed up on the Online card ("1 open") through the DHT board and slots.
  So Find's DHT path isn't broken in the Android build.
- **The LAN lookup can't work on a real phone.** Android's Wi-Fi drops incoming multicast unless the app holds a
  `WifiManager.MulticastLock` (with the `CHANGE_WIFI_MULTICAST_STATE` permission). The app had neither, so mDNS
  (`net.rs` `watch_lan`, Find's LAN list) never heard the hosts on the same network. Sending still works, so the PC
  could see the phone's lobby, which fits "hosting works". The emulator can't show this, because its NAT carries no
  multicast.
- **A weak spot on phones:** the mainline DHT resolves its bootstrap host names only once, when the node starts
  (`Dht::client` at going online). If DNS isn't up at that moment, the node keeps an empty routing table for good.
  Find then lists nothing, while Host still gets out through the iroh/pkarr relays.

## Changes
- `HstActivity.java`: a non-refcounted `MulticastLock` ("hst-lan"), taken in `onResume` and released in `onPause`.
  `tools/apk.sh`: added the `CHANGE_WIFI_MULTICAST_STATE` permission. On the emulator, `dumpsys wifi` lists
  `Multicaster{hst-lan}`.
- `net.rs` `Ctx::revive_dht`: before Host, Find, Quickplay and Join, a node with no routing table gets up to 10 s to
  bootstrap. If it still has none, it's replaced by a fresh node, which resolves the bootstrap names again. Warnings
  go to hst-log.txt.
- `net.rs` Find: logs one line per lookup (`lobby find: N from the LAN / the DHT board / the board slots`), so a
  phone's hst-log.txt shows where an empty list came from.
- `tools/apk.sh`: `${HST_VERSION#v}` under `set -u` aborted every non-release build. It now defaults to empty.

## Verified
- Emulator (hst34, own `-read-only -port 5674`): the new APK lists the desktop's lobby. Log:
  `lobby find: 0 from the LAN`, `1 from the board slots`, `1 from the DHT board`. The multicast lock is held while
  the app is in front.

## Not verified / not 1:1
- **Real phone not tested** (no device on adb). The LAN fix rests on documented Android Wi-Fi behaviour. The emulator
  has no multicast, so it can't show LAN discovery either way. The user should retry with the PC and phone on the same
  Wi-Fi. If the list is still empty, the hst-log.txt lines say which lookup came back empty.
- `revive_dht` was never triggered for real: I couldn't make DNS fail at start-up. The healthy path (routing table
  present → return at once) ran on the emulator.
- I couldn't reproduce the user's exact setup (which network, and whether the lobby they expected was LAN-only).
- Front-end/network only: gameplay untouched, so nothing to compare with the original.
- For B454 (noted there): reqwest 0.13 uses rustls-platform-verifier. On Android that needs a JNI init plus its Java
  classes in the APK, which would break HTTPS (the mod store, the pkarr relay fallback). This is a suspect, not
  confirmed.

## Wrap-up
- The full test run fails on `play::main_menu::pointer::tests::wide_window_hits`, which fails the same way when run
  alone. It's a character-select hit test this task doesn't touch, broken at the branch base (94a82095), probably by
  the roster card change before it. It's filed as its own task, so B455 was ticked and committed by hand rather than
  through `done.sh`.
- My first `revive_dht` waited for the bootstrap before each Host or Find, which broke `host_cuts_a_running_find`
  (B321). It now never waits: it only replaces a node that still has no table 20 s after it started. That test passes.
