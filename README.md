# Hot Shots Tennis — Rust rewrite

A from-scratch rewrite of the PlayStation 2 game **Hot Shots Tennis** (US, 2007), known as **Everybody's Tennis**
in Europe and **Minna no Tennis** (みんなのテニス) in Japan, in Rust with the [Bevy](https://bevyengine.org) engine.
The goal is the same game: the same ball physics, rules, timing and AI, ported from the original and checked
frame by frame against the real game, with better rendering on top.

Work in progress. `TODO.md` is the plan and shows what is done.

## You must provide the game disc

**This repository contains no game data**: no models, textures, animations, sounds, music or program code from the
original. Everything the game shows or plays is read **at runtime from your own copy of the disc**. Without it,
nothing runs.

What you need:

| | |
|---|---|
| **Disc** | Hot Shots Tennis, **US release, version 1.00, serial SCUS-97610** (PS2) |
| **Format** | A disc image (`.iso`) you made from **a disc you own** |
| **Where** | Anywhere; you pass its path on the command line. The examples assume `Hot Shots Tennis (USA).iso` in the repository folder, which `.gitignore` keeps out of git |

- **Other regions don't work yet.** Everybody's Tennis (Europe), Minna no Tennis (Japan) and other versions have
  different program data. The game checks the disc (`SYSTEM.CNF` boot file `SCUS_976.10` and the size and header of
  `ZZBIN/GAME.BIN`) and refuses anything else with `unsupported disc`.
- **Don't ask for, share or commit disc images or anything extracted from them.** Nobody here can provide one.
- **You don't need a PS2 BIOS, an emulator or any other files** to play. Those are only for the research tools
  (see *Development* below).

## Build and play

Requires a recent stable [Rust](https://rustup.rs) toolchain (edition 2024). Developed and tested on Linux. Bevy
also supports Windows and macOS, but they are untested here.

```
cargo run --release -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```

- `--stage 01..11`: which court to load (with its props) from the disc.
- `--singles`: 1 vs 1 instead of doubles.
- `--chars 6,1,2,3`: characters on court, in player order (the default is Carol for player 1).
- `HST_AUTOPLAY=1`: every player is a CPU.

**Players:** player 1 uses the keyboard or the first controller. In doubles, a second controller plays player 1's
partner (P1 + P2 vs 2 CPUs). In singles it plays the opponent. Every other player is a CPU.

| Action | Keyboard | Controller |
|---|---|---|
| Move (aim while swinging) | WASD | Left stick / d-pad |
| Topspin | J | A |
| Slice | K | B |
| Flat | I | X |
| Lob | L | Y |
| Drive | U | RB |
| Serve (toss, then hit) | J / Space | A / Start |
| Camera (original / free) | C | Select |
| Turn free camera | Arrow keys | Right stick |

## Development

- `crates/hst-data`: readers for the disc and its formats (ISO, XB archives, TIM2 textures, models, motions,
  layouts).
- `crates/hst-sim`: the game logic (ball physics, collision, rules, serve, contact search, camera), engine-free.
- `crates/hst`: the Bevy app.
- `tools/`: scripts to drive and record the original game in PCSX2 over PINE, for verification.

`cargo test` runs everywhere. Tests that need the disc or recordings of the original game look in the repository
folder and in `context/fixtures` (or `HST_FIXTURES`), and **skip themselves when those are missing**. The recordings
are made from the original game and are not distributed.

To verify against the original, you also need PCSX2 and your own PS2 BIOS, since the tools drive the real game.
Reverse-engineering notes and dumps live in `context/`, which is git-ignored and never published. Shipped code
describes behaviour only. The single exception is `hst-data::exe`, which reads tuning values from your disc's
program file at documented offsets.

## Legal

This is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or Clap
Hanz. Hot Shots Tennis, Everybody's Tennis and Minna no Tennis are trademarks of their owners. No copyrighted game
data is included or distributed. You must own the game to use this.
