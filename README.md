# Hot Shots Tennis — Rust rewrite

A from-scratch rewrite of the PlayStation 2 game **Hot Shots Tennis** (US, 2007), known as **Everybody's Tennis**
in Europe and **Minna no Tennis** (みんなのテニス) in Japan, in Rust with the [Bevy](https://bevyengine.org) engine.
The goal is the same game: the same ball physics, rules, timing and AI, ported from the original and checked
frame by frame against the real game, with better rendering on top.

Work in progress. `PLAN.md` holds the rules, `plan/TODO.md` the open tasks, `plan/DONE.md` the finished ones; `plan/REFERENCE.md` lists what works.

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

- **Other regions don't work.** Everybody's Tennis (Europe), Minna no Tennis (Japan) and other versions have
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
| Slice / Serve | K | B |
| Lob | L | Y |
| Drive | U | RB |
| Camera (original / free) | C | Select |
| Turn free camera | Arrow keys | Right stick |

### In the browser

`tools/web.sh` builds the browser version into `web/` (a WebGPU build, and a WebGL2 one for browsers without WebGPU;
the page picks). It needs the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` at `Cargo.lock`'s wasm-bindgen
version and `wasm-opt` (binaryen). Serve `web/` over HTTPS, or `python3 -m http.server -d web` on localhost. The page
asks for your ISO once and keeps the imported disc in the browser's private storage (OPFS); `?forget` drops it. Sound
starts on the first click or key. The repo's GitHub Pages workflow (`.github/workflows/pages.yml`) publishes the same
page; no game data is hosted.

## Texture replacements

Upscaled or edited textures are read at runtime from `mods/texture-replacements/` beside the ISO, which is not part
of the repository. Each texture is taken from the first match:

1. **Key-named files** (`<name>-<key>.png`, in any subfolder or flat): your own textures, edited or upscaled, at
   any size and with straight (0–255) alpha. A file is matched by the part of its name after the last `-`.
   Changed files reload while the game runs.
2. **A PCSX2 texture pack**: the hash-named PNGs PCSX2 dumps or loads, at the folder's top level. The port computes
   PCSX2's hash for each disc texture, so a pack made for the original game works as is.
3. The disc.

```
Hot Shots Tennis (USA).iso
mods/textures-src/...             (written by --dump-textures)
mods/texture-replacements/...     (yours: upscaled/edited key-named files, and/or a PCSX2 pack)
```

**Making your own:** `hst <iso> --dump-textures` writes every disc texture, at its native size, to
`mods/textures-src/` in readable folders (`COURT/05/...-<key>.png`). Edit or upscale those and put the results in
`mods/texture-replacements/` under the same file names. A flat folder is fine, because only the key matters.

To upscale them all at once, use the chaiNNer chain in `tools/chainner/upscale.chn`. It reads every PNG under
`mods/textures-src/`, runs a 4× model and writes the results to `mods/texture-replacements/`. The file has no folders or model
set, so after opening it in chaiNNer:

- Set *Load Images* to your `mods/textures-src` and *Save Image* to your `mods/texture-replacements`.
- Set *Load Model* to your own upscaling model. The chain was made for 4xHDcube4 (a paid model, not included),
  but any 4× ESRGAN-style model works.

**Character mods** keep their textures inside their `.glb` files. `python3 tools/upscale_mods.py MODEL.pth`
upscales every mod under `mods/` at once with the same chain, run headless (no GUI, no paths to set), into each
mod's `textures/upscaled/`; the "upscaled textures" setting then draws them. It skips textures already done, so
rerun it after adding or rebuilding mods.

The startup log line `textures: N PCSX2-named, M key-named in mods/texture-replacements` shows what was found. Bundling upscaled
textures of our own into the remaster is planned (`PLAN.md` M5). Like everything else from the disc, they would be
made on your machine, not shipped.

## Character mods

Custom characters go in `mods/<id>/` beside the ISO (`mod.json` and one `.glb` per costume) and show up in the
character select. The mod standard, a tutorial for making your own, and the tools that extract the other Hot Shots
games' characters and package them as mods are in the mods repo:
<https://github.com/FIZL0/hot-shots-tennis-rust-mods> (`standard/`, `tools/`). The game checks models against its copy
of the skeleton, `crates/hst/hst_skeleton.json`; keep it equal to the mods repo's `standard/hst_skeleton.json`.

## Online play

Main menu → Online: host a lobby, find lobbies, quick play, or join with a code (paste it, or scan the host's QR).
Lobbies are found over the public BitTorrent DHT. Players connect peer to peer with [iroh](https://iroh.computer), and
a relay steps in when a direct path can't be made. In the browser everything goes through the relay: join by code
(paste the code on the page with Ctrl+V, or open the page with `?code=<code>`) or host; Find isn't there (no UDP in a
browser), and Quick play hosts. Browser and desktop players can play together.

Browsers on one network can also play with no internet and no server (WebRTC): the host sets the relay to Off and
hosts; each player scans the host's QR with their phone camera (it opens the game with the code), joins, and shows a
QR of their own, which the host scans in turn (the tab it opens hands the code to the game tab), or the codes are
pasted both ways. Browser-only: desktop builds can't join these lobbies.

LAN play needs no internet: put everyone on one network (a router, or one player's hotspot with mobile data off) and
set the relay to Off. Find lists the lobbies on that network first (mDNS), and a host's code holds its LAN address,
so joining by code works there too. Your firewall must let mDNS (UDP 5353) and the game's UDP port through.

Settings live in `online.txt` beside the ISO. The Online screen writes this file, or you can edit it yourself:

```
name = Tennis            # your lobby's name
relay = n0               # n0 (iroh's public relays), own (relay_url), off (direct only)
relay_url = https://relay.example.org
```

To run your own relay, install it with `cargo install iroh-relay --features server`. On a machine with a public IP and
a domain, run it with a TLS config (`iroh-relay --config-path relay.toml`, see iroh-relay's docs). Then set
`relay = own` and `relay_url = https://your.domain` on every player. `iroh-relay --dev` serves plain HTTP on
`:3340` for local tests.

The same flow runs in a terminal without the window. Each command prints the events:
`hst net host|find|quick [doubles] [players]`, `hst net join <code> [players]`, each with an optional
`--relay n0|off|<url>`.

## Development

- `crates/hst-data`: readers for the disc and its formats (ISO, XB archives, TIM2 textures, models, motions,
  layouts).
- `crates/hst-sim`: the game logic (ball physics, collision, rules, serve, contact search, camera), engine-free.
- `crates/hst`: the Bevy app.
- `tools/`: scripts to drive and record the original game in PCSX2 over PINE, for verification.
- `research/`: our reverse-engineering scripts and research journal. `research/README.md` explains how to rebuild
  the private analysis folder (decompilation, save states, recordings) from your own disc.

`cargo test` runs everywhere. Tests that need the disc or recordings of the original game look in the repository
folder and in `context/fixtures` (or `HST_FIXTURES`), and **skip themselves when those are missing**. The recordings
are made from the original game and are not distributed.

To verify against the original, you also need PCSX2 and your own PS2 BIOS, since the tools drive the real game.
Reverse-engineering notes and dumps live in `context/`, which is git-ignored and never published. Shipped code
describes behaviour only. The single exception is `hst-data::exe`, which reads tuning values from your disc's
program file at documented offsets.

Claude Code sessions in this repo auto-compact at 200k tokens of context (`autoCompactWindow` in
`.claude/settings.json`): past that, every turn re-reads the whole conversation and most usage goes there. For a big
change that needs more in view at once, raise it for that session only: `CLAUDE_CODE_AUTO_COMPACT_WINDOW=500000 claude`
(the variable overrides the setting), or edit the setting and put it back afterwards.

## Legal

This is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or Clap
Hanz. Hot Shots Tennis, Everybody's Tennis and Minna no Tennis are trademarks of their owners. No copyrighted game
data is included or distributed. You must own the game to use this.
