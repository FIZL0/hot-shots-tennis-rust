# M105 — Android hst-log.txt

Android now writes `hst-log.txt` into the app's external files dir
(`/sdcard/Android/data/io.github.fizl0.hst/files/`). The file is fresh on each launch, and the previous launch's log is
kept as `hst-log-previous.txt`. That way a crash log survives the user reopening the app, which iOS doesn't do.

How it works (`crates/hst/src/android.rs` `log_to_files`, called first thing in `android_main`):
- android-activity already points fds 1/2 at a pipe that it forwards to logcat. We save that fd with `dup(2)` and put
  our own pipe over fds 1 and 2. A thread `hst-log` copies the pipe into the file and on to the saved fd. So stderr,
  println and Rust's panic messages (from the default hook) go to the file and still reach logcat.
- Engine log: `LogPlugin::fmt_layer` = plain-text (no ANSI) fmt layer writing straight to the file. It replaces bevy's
  default coloured stderr layer, which would have doubled every line in the file and in logcat. logcat still gets
  the engine log through bevy's AndroidLayer.
- As on iOS: `HST_FPS=1` (the 5 s profiler lines), a version line, `exit() called` via atexit, and a frame/lifecycle
  heartbeat (`android::heartbeat`). The no-disc start page uses the same layer.
- Fatal signals (ABRT/SEGV/BUS/ILL/TRAP/FPE): the handler writes one line straight to the file (async-signal-safe
  `write`), then calls the handler that was there before (`sigaction`-saved, i.e. debuggerd's), so tombstones still
  get written.

## Verified (x86_64 emulator, hst34 read-only on port 5590, APK from `HST_ABIS=x86_64 tools/apk.sh`)
- Normal launch into the menu: version line, engine log without escape codes (0 ESC bytes), fps/stall/draw lines,
  heartbeat. `hst-log-previous.txt` is rotated on relaunch.
- A temporary trigger (since removed) forced each crash type:
  - Main-thread panic: the panic message is in the file.
  - `raise(SIGSEGV)`: `fatal signal: SIGSEGV`, and logcat still shows debuggerd's `Fatal signal 11` + backtrace.
  - Thread panic followed by abort: the panic message and `fatal signal: SIGABRT`.
  - logcat still shows `RustStdoutStderr` lines in every case.

## Not verified / not 1:1
- Not tested on a real phone, only the emulator (no device attached). The code path is the same.
- Gameplay is not touched (logging only), so there's nothing to compare with the original.
- The signal line can come before stderr lines still sitting in the pipe, because it's written directly while the
  pipe thread may not have caught up yet. Seen in the SEGV case: it landed above the version line. All the lines
  are there, but the order can be off right at a crash.
