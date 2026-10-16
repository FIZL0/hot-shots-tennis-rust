# B503 — every compiler and clippy warning, fixed

Built and clippy-checked: `cargo build/clippy --workspace --all-targets` (x86_64 Linux), `-p hst` for
`wasm32-unknown-unknown` (with and without `--features webgpu`), `-p hst-sim` for wasm32, the Android `.so`
(`-p hst --example hst --features android --target aarch64-linux-android`, NDK clang as in `tools/apk.sh`), iOS
(`tools/ios-build.sh clippy -p hst`) and `x86_64-pc-windows-gnu`. About 540 unique warnings at the start. Now none
from our code on any of those targets, except the one Cargo manifest warning below. Logs: `context/b503/`.

## What was done
- **Lint config** (`[lints.clippy]` in each crate's Cargo.toml, with the reason written there):
  - `type_complexity` and `too_many_arguments` are off in the three Bevy crates. Systems take each query and
    resource as a parameter, so those lints fire on every ordinary system.
  - `neg_cmp_op_on_partial_ord` is off in hst-sim. `!(a < b)` is the EE's c.lt.s + bc1f, which is taken for NaN.
- **clippy --fix**, with each change checked by hand:
  - compound assigns, let-chains (laid out in the repo's style, not rustfmt's), `is_multiple_of`, casts that did
    nothing, precedence parens.
  - `chunks_exact(N)` → `as_chunks::<N>()`. Where `--fix` broke the follow-on code (`try_into` on `&[u8; 4]`,
    `Vec<&[u8]>` collects, `.map(Frame)`), it was fixed by hand.
  - ps2-float's `F` `*=`/`+=`/`-=` go through the same EE ops as `a = a * b`, so the sim stays bit-exact (checked
    in `fpu.rs`'s `fpu_op!`). `0.999_000_013` → `0.999` is the same f32.
- **By hand**:
  - index loops → iterators/zip.
  - type aliases for the long tuple types (shade `Tri`, test helpers).
  - `const { assert!(..) }` for the constant asserts.
  - `std::slice::from_ref`, `Reverse` sort keys, `checked_div`.
  - `matches!`, and merged `if` arms that had identical blocks.
  - struct-update syntax in place of assigning fields after `Default`, and a `.fold` that can't short-circuit
    (`&`).
  - `Dive::len` → `Dive::lock_frames`, so it isn't read as a length.
  - net.rs's rtc helpers are `pub(in crate::net)`.
- **Kept allows, each with a one-line reason**:
  - 7 ported sim routines that take the game's own argument lists (`too_many_arguments`).
  - `mdl.rs`'s per-vertex loop, which walks five parallel arrays.
  - `large_enum_variant` on net's `Guest` and on the menus' `Effect`/`online::Fx`. Those values live for one
    frame, and boxing would only add an allocation.
  - `cfg_attr(android, allow(dead_code))` on `main`/`update::cleanup`. The Android `.so` starts at `android_main`.
  - `float_arithmetic` on one test's host-built input.
- **Dead code left over from the old immediate-mode menus** (no callers since B397/B449's redesigns), removed:
  - child.rs's `draw`/`rects`/sizes, `flame::draw` (Confirm has no VS any more; the flame still steps for its
    `rand()` draws), `help_box`, `TILE_*`, `Tex::Hand`, `court_mod_sub`, `entries_len`, mods screen `TOP`.
  - Helpers that only tests use are now `#[cfg(test)]` (`seat_tag`, `rows_locked`, `shown`).
  - `online.rs`'s `find_first_then_pick_in_the_lobby` had lost its `#[test]`. It's back, and it passes.
- **cfg-dependent fixes** that only the other targets showed:
  - clippy's `match &*can` fix broke wasm (`web::vibration()` is a `String` there).
  - wasm-only: `jobs.rs` `mut`, `net.rs` `return` in a cfg block, `previews::cache_dir`, the self-updater's
    constants and `Effect::Quit`.
  - iOS: the `/proc` CPU read.
  - Android: `OLD` is now read by copy instead of `&*(&raw const ..)`.
- play.rs: local edits only, nothing moved or reformatted.

## Checks
- `cargo clippy --workspace --all-targets` and `cargo build --workspace --all-targets`: no warnings.
- `HST_NETPLAY=1 tools/check.sh -p hst-play netplay` (30 tests), `-p hst-play rollback` (5) and `-p hst-base net`
  (37) all pass. The full suite, replay tests included, runs in `tools/done.sh`.

## Not verified / not 1:1
- Cargo's `main.rs found to be present in multiple build targets` (the `hst` bin and the Android `[[example]] hst`
  both use it) is still there. I tried a wrapper root with `include!("main.rs")`, but it can't take main.rs's inner
  `//!` docs and `#![..]` attributes. A clean fix means giving `hst` a lib target for the `.so`, which is a
  restructure and out of scope here.
- `gilrs-core` (a dependency) has 4 warnings on Android/iOS. They are not ours to fix.
- The Android, iOS and Windows builds were checked with clippy only. They were not linked or packaged
  (`tools/apk.sh`, `tools/ios-ipa.sh`), and none of them was run on a device.
- Nothing here needs PCSX2: these are compile-time changes, and gameplay exactness is covered by the replay tests.
