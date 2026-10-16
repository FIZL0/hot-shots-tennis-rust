# Developing and modding

Building from source, manual texture replacements, mod authoring, online settings by hand and the test setup.
Players don't need any of this: see the [README](../README.md).

## Build from source

Requires a recent stable [Rust](https://rustup.rs) toolchain (edition 2024). Developed and tested on Linux. Bevy
also supports Windows and macOS, but they are untested here.

```
cargo run --release -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```

- `--stage 01..11`: which court to load (with its props) from the disc.
- `--singles`: 1 vs 1 instead of doubles.
- `--chars 6,1,2,3`: characters on court, in player order (the default is Carol for player 1).
- `HST_AUTOPLAY=1`: every player is a CPU.
- `HST_TEXTURE_RELOAD=1`: re-read changed `mods/texture-replacements/` files while running (texture authoring).

The released builds (built with `--features ship`) ignore the `HST_*` variables except `HST_TEXTURE_RELOAD`,
`HST_FPS` (frame log) and `HST_TRACE` (menu input trace); `cargo run` builds honour them all.

Controls: see the [README](../README.md#controls).

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
   With `HST_TEXTURE_RELOAD=1` changed files reload while the game runs (off by default: the folder is read once).
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

**Character, court and umpire mods** keep their textures inside their `.glb` files. `python3 tools/upscale_mods.py MODEL.pth`
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
<https://github.com/FIZL0/opentennis-mods> (`standard/`, `tools/`). The game checks models against its copy
of the skeleton, `crates/hst/hst_skeleton.json`; keep it equal to the mods repo's `standard/hst_skeleton.json`.

## Online settings by hand

Players connect peer to peer with [iroh](https://iroh.computer); lobbies are found over the public BitTorrent DHT,
and a relay steps in when a direct path can't be made. In the browser everything goes through the relay.

Settings live in `online.txt` beside the ISO. The Online screen writes this file, or you can edit it yourself:

```
name = Player            # your name (on your lobby seats; a host's lobby is listed under it)
relay = n0               # n0 (iroh's public relays), own (relay_url), off (direct only)
relay_url = https://relay.example.org
```

Online → Relay cycles n0 (public relays), Off, your own relay once one is set, and last, for advanced players,
Custom...: OK on it opens a text field for your own `iroh-relay`'s URL (type or paste it, `https://…` or `http://…`).
Save checks that it parses, switches to it and keeps it in `online.txt`; Back skips on to n0. Every player in a
lobby needs the same relay. This works in the browser too.

To run your own relay, install it with `cargo install iroh-relay --features server`. On a machine with a public IP and
a domain, run it with a TLS config (`iroh-relay --config-path relay.toml`, see iroh-relay's docs). Then type
`https://your.domain` under Relay → Custom... (or set `relay = own` and `relay_url = https://your.domain` in
`online.txt`) on every player. `iroh-relay --dev` serves plain HTTP on
`:3340` for local tests.

The same flow runs in a terminal without the window. Each command prints the events:
`hst net host|find|quick [doubles] [players]`, `hst net join <code> [players]`, each with an optional
`--relay n0|off|<url>`. A guest is ready once it joins. The host starts when every seat is taken, or with `--bots` as
soon as one guest is in and ready, and the computer plays the open seats.

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

