# M91 — native iOS IPA built on Linux

User 2026-10-14: the browser build runs poorly on an iPhone 13 Pro; wants a native IPA they sign themselves. No Mac.

- `cargo check -p hst --target aarch64-apple-ios` stops at the first C build script (`tracing-oslog`, needs `os/log.h`):
  nothing builds past that without Apple's iOS SDK, and the SDK only ships inside Xcode.
- Linker: rustup already ships `ld64.lld` (`rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/`); clang is installed.
- `tools/ios-sdk.sh Xcode.xip` extracts `iPhoneOS.sdk` to `~/.local/share/hst-ios/` with bsdtar + an inline pbzx
  decoder + cpio (no xar/pbzx packages, no sudo). Checked on a synthetic xip (mixed xz/raw chunks, SDK symlink, junk
  filtered out); not yet on a real Xcode.

Next: user downloads Xcode .xip (free Apple ID) → extract → check/build with SDKROOT + clang/ld64.lld → fix
iOS-incompatible deps (rfd, gilrs) → bundle .app/Payload → unsigned .ipa.
