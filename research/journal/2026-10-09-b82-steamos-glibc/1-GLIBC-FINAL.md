# B82 — Linux build runs on SteamOS (older glibc)

Problem: `tools/build.sh linux` linked against the dev machine's glibc 2.44 (binary needed GLIBC_2.44); a player's
SteamOS couldn't start it.

SteamOS glibc (DistroWatch package table, 2026-10-09): 3.5.7 → 2.37, 3.7.7 → 2.41, 3.8.14 → 2.44. Steam Runtime
"sniper" (Debian 11) → 2.31. Target chosen: **2.31**, below every SteamOS 3.x and equal to the Steam Runtime.

Build: `cargo zigbuild --target x86_64-unknown-linux-gnu.2.31` (cargo-zigbuild 0.23.4, zig 0.17 from the `ziglang`
pip package). Linking failed with `--no-allow-shlib-undefined` because the host's libasound.so references
GLIBC_2.34+ symbols; those belong to the host's library, not ours, so the build passes `--allow-shlib-undefined`.
The binary then needs at most **GLIBC_2.30**; `build.sh` prints that and fails if above 2.31.
musl not tried: the task notes say a static musl binary can't load the system Vulkan/ALSA/udev libraries.

Docker isn't usable without root here, so tested with the locally installed Steam `SteamLinuxRuntime_sniper`:
- `SteamLinuxRuntime_sniper/run -- hst <iso> --play --shot …`: Vulkan (RADV) initialized, match started, HUD +
  players drawn (screenshot in `context/b82_sniper.png`). pressure-vessel uses the host's glibc when it is newer, so
  this proves the runtime's libraries, not glibc 2.31.
- glibc 2.31 itself: the sniper platform's own `ld-linux-x86-64.so.2` + libc 2.31 (staged under their sonames),
  `LD_BIND_NOW=1`: the new binary resolves every symbol and boots into Bevy; the old dist binary won't load.
Not tested on real Deck hardware (none here); the binary needs nothing above 2.30 and SteamOS 3.5+ ships ≥ 2.37.

## Follow-up: build in the sniper SDK container
Docker set up (user in group `docker`). `tools/build.sh linux` now builds inside
`registry.gitlab.steamos.cloud/steamrt/sniper/sdk` (glibc 2.31, native gcc/ld against sniper's own libs, so no
`--allow-shlib-undefined`); rustup stable is installed into `~/.cache/hst-steamrt`, build dir `target/steamrt`.
zigbuild dropped. Result needs GLIBC_2.30; same two checks pass (match in `SteamLinuxRuntime_sniper/run`,
`context/b82_sniper2.png`; eager load under sniper's glibc 2.31). Gotcha: `rustup update` as a uid with no passwd
entry fails (`getpwuid_r`) and killed the `sh -e` step; it isn't needed.
