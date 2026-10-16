# B418 — Android: HTTPS through the phone's own certificate check

## Before
reqwest's rustls backend verifies certificates with `rustls-platform-verifier`, which on Android must be initialised
over JNI and needs its Java classes in the APK; neither was there, so every HTTPS request panicked with "Expect
rustls-platform-verifier to be initialized". B454 had already worked around it: Android clients used Mozilla's
bundled roots (`webpki-roots`), so no panic, but the phone's own trust store (user-installed CAs, revocation) was
ignored.

## Fix
- `android.rs` `init_tls()`: called from `android_main` right after `ANDROID_APP` is set. It attaches through jni 0.22
  (the verifier's version, as `jni22`) and calls `rustls_platform_verifier::android::init_with_env(env, activity)`.
  0.7.1 has no `init_hosted`, so this is the equivalent call: the VM, the activity as the context, and the
  activity's class loader.
- `tls()` now builds rustls on ring with `rustls_platform_verifier::Verifier` once the init has succeeded. If the
  init fails it falls back to B454's webpki roots and logs a line. `modstore::client()` is unchanged; every reqwest
  call (store, update check, pkarr relay) still goes through it.
- `tools/apk.sh`: downloads the verifier's AAR for the `rustls-platform-verifier-android` version in Cargo.lock
  (0.2.0, from the project's maven-archive branch on GitHub) and kotlin-stdlib 2.2.10 (Maven Central) once into
  `~/.cache/opentennis-apk`, both sha256-pinned. The script exits if Cargo.lock moves to another verifier version.
  The dex is now built with R8 (it ships inside build-tools' d8.jar), with keep rules for our activity and
  `org.rustls.platformverifier.**`, so only the Kotlin stdlib classes the verifier uses are included. The result is
  a 22 KB classes.dex (it was 3 KB).
  The AAR's `network_security_config.xml` is now the app's `android:networkSecurityConfig`: it allows cleartext only
  to the CAs' CRL hosts, for the verifier's revocation checks.

## Verified (APK on the hst34 emulator, own `-read-only -port 5604` instance)
- The release APK (arm64 + x86_64) builds; dexdump shows `org.rustls.platformverifier.*` plus a few kotlin classes.
- The app starts with no panic in `hst-log.txt`.
- A temporary probe build (not committed) ran these through `modstore::client()`:
  - platform verifier active (`init_tls` succeeded);
  - the update check's request (`releases/tags/beta`): 200, `target_commitish` read;
  - `self-signed.badssl.com`: rejected (UnknownIssuer, by Android's CertPathValidator);
  - `expired.badssl.com`: rejected (Expired);
  - store listing: `mods-store.json` written with 9 mods from the PR refs;
  - `modstore::install` of `gag_ump01_nate` (refs/pr/66, 654 KB): installed with its `store.json` (sha256 checked
    by the installer), `umpire.glb`, `voice/`, `previews/`.
- `tools/check.sh -p hst` passes; `cargo check --features android` for x86_64-linux-android is clean.

## Not verified / not 1:1
- Not driven through the menu UI (the update banner, the Browse mods screen and its Install button). This headless
  software-GL emulator returns black `screencap` frames, so I couldn't navigate by touch. The probe called the same
  functions the menu calls (`modstore::client`, `list`, `install`).
- Not run on a physical phone or an arm64 device; only the x86_64 emulator image.
- Revocation via the bundled network security config: not exercised (no revoked-certificate test host was tried).
- Front-end/network code, nothing to match against the original game.

## Wrap-up note
`tools/done.sh`'s full test run failed only `play::court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match`
("0 of the mod's 0 people placed"). The test reads `mods/hst-mods/gag_c01_yamate`, a symlink into
`../opentennis-mods/out`. Another session re-exported that mod at 23:04, during this run, without its `npcs.json`;
its gallery exporter is being removed in that repo's working tree. That has nothing to do with this change, which
only touches Android code. `tools/check.sh -p hst` had passed 457/457 at 22:5x, before the re-export. audit and the
sim clippy gate pass; ticked and committed by hand.
