# B567: the iOS 1.0 app doesn't offer 1.1.0

## What was checked
- The release side is fine: `releases/latest` answers `v1.1.0` (not a pre-release, not a draft) with `opentennis-ios.ipa`.
- The shipped v1.0.0 IPA (downloaded, `strings` on its binary) has `v1.0.0` baked in (`HST_VERSION`) and the
  `releases/latest` URL, so the check is in that build. `found()` with no asset for the platform (iOS, Android) still
  offers the release page (Download), already covered by the updater test.
- Timeline: v1.0.0 was published 00:38 UTC, v1.1.0 07:45 UTC, the report came 07:51 UTC.

## Cause
The check ran once at start, then `thread::sleep(30 min)` in a loop. iOS keeps an app suspended in the background
for hours or days instead of quitting it, and a sleeping thread's (monotonic) clock doesn't advance while the app
is suspended or the phone sleeps. An app opened the night before (when 1.0 was the newest) and brought back after
1.1.0 came out kept waiting out the rest of its 30 minutes of *running* time, so no offer showed. Android and a
laptop that sleeps have the same problem.

## Change
- `update.rs`: the next check is due half an hour after the last **by the wall clock** (`due()`), polled every 5 s,
  so a resumed app asks again within seconds of coming back. Each check's outcome is logged at info
  (`update check: v1.1.0 is out` / `v1.0.0 is the latest`), so a phone's `hst-log.txt` shows what it saw.
- The Update row's words on phones: they said "replace this game… folders next to it" and "Builds: Linux, Windows";
  iOS now says to install the release's .ipa over the app the way it was sideloaded (AltStore, Sideloadly), Android
  the .apk. An IPA can't update itself (the app bundle is read-only and unsigned), so Download → release page is the
  update path there.
- Test `checks_again_half_an_hour_later_by_the_wall_clock`; `tools/ios-build.sh check -p hst` builds.

## Not verified / not 1:1
- Not run on an iPhone (no device here): that suspension was the cause on the user's phone is inferred from the
  timeline and the code, not seen. If a freshly started 1.1+ app still shows nothing when a newer release is out,
  its `hst-log.txt` now has the `update check:` line (an HTTPS error there would point at the iOS certificate check,
  reqwest's platform verifier).
- The 1.0 IPA itself can't be fixed after the fact: a 1.0 app shows 1.1.0 once it is quit and started again
  (swipe it away in the app switcher).
- Not 1:1 with the original: it has no updater (n/a).
