//! The Android NativeActivity `.so` (`[[example]] hst` in Cargo.toml, `tools/apk.sh`): the same crate as the binary.
//! Its own root file, so Cargo doesn't warn about main.rs being in two targets.
include!("main.rs");
