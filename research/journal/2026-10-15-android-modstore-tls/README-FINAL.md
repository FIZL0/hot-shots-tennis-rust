# B454 — Android: the mod store can't be read

## Cause
reqwest 0.13 (rustls, no explicit roots) checks server certificates with `rustls-platform-verifier`. On Android that
verifier calls into Java (`org.rustls.platformverifier.CertificateVerifier`) and has to be initialised through JNI
first; our NativeActivity APK has neither the init call nor the Kotlin classes. The first HTTPS handshake then panics
("Expect rustls-platform-verifier to be initialized") inside the spawned tokio task, so `modstore::list` never sends
an `Event::Listing`: the store stays empty/unread. The same panic hit the update check (`update.rs`) and the short
lobby codes' pkarr relay (`net.rs`). Not the cause: INTERNET permission (in the manifest), storage paths.

## Fix
- `android.rs` `tls()`: a rustls `ClientConfig` on ring with Mozilla's roots (`webpki-roots`, already built for
  iroh-relay), ALPN http/1.1, cached.
- `modstore::client()` (now `pub(crate)`) hands it to reqwest on Android (`tls_backend_preconfigured`); every reqwest
  call (store GETs, range downloads, HF API, update check, pkarr relay get/put) now goes through `client()`.
  Desktop, iOS and wasm keep their old behaviour.

## Verified
A probe binary with the same reqwest 0.13.5 and the same config, cross-built for x86_64-linux-android and run on the
hst34 emulator (`-read-only -port 5594`):
- default `reqwest::Client::new()` → `JoinError::Panic("Expect rustls-platform-verifier to be initialized")` (the bug);
- the `tls()` client → 200 from `huggingface.co/api/datasets/FIZL0/opentennis-mods/tree/main`;
- the `tls()` client → rejects `self-signed.badssl.com` (certificates are still checked).
`cargo check` for x86_64-linux-android (`--features android`) and wasm32 passes; `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- The full APK wasn't rebuilt and driven to Browse mods on a device/emulator (needs the disc imported on the
  emulator and touch navigation); the probe exercises the identical client path instead.
- Roots are Mozilla's bundled list, not the phone's own store: user-installed CAs (e.g. a corporate proxy) aren't
  trusted. Front-end/network code, nothing to match against the original.
