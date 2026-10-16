# B420 — mobile mod loading: close instead of restart

The iOS and Android apps can't start themselves again, so the desktop Reload (spawn self + quit) can't work there.

- `mods_screen.rs`: `View::app` (`PhoneApp`, true on `target_os = "ios" | "android"`). On the app:
  - the main menu's popup asks "Mods changed. / Close the game to use them?" with **Close game** / Later; its
    description says the app can't restart itself and to open it again (`ask_words`);
  - **Close game**, and the Mods page's Reload pills (Installed, Browse's heading), call `reload(app=true)`, which
    ends the process at once with `std::process::exit(0)` — a Bevy `AppExit` doesn't end an iOS app. Installs and
    removals and settings are already on disk, so nothing is lost; the player opens the app again;
  - the Reload pill's description says it closes the game (`reload_said`).
- Desktop and web unchanged.
- Test `app_popup_closes_the_game` (words, Later still only closes the popup). Shot: `HST_MENU=reloadmods HST_APP=1
  tools/shot.sh …` (landscape + portrait 720×1280 checked; in `context/B420/`).

## Not verified / not 1:1
- Not run on an iOS device/simulator or Android emulator: the change is `cfg!`-only (no target-gated code), shown on
  desktop via `HST_APP=1`. That `exit(0)` behaves cleanly on both (no crash report on iOS; Android's task may stay
  in Recents and relaunch fresh) is assumed, not observed.
- Not a gameplay feature: no comparison with the original (front-end, ours to design).
