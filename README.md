# Hot Shots Tennis — Rust rewrite

A from-scratch rewrite of the PlayStation 2 game **Hot Shots Tennis** (US, 2007), known as **Everybody's Tennis**
in Europe and **Minna no Tennis** (みんなのテニス) in Japan, in Rust with the [Bevy](https://bevyengine.org) engine.
The goal is the same game: the same ball physics, rules, timing and AI, ported from the original and checked
frame by frame against the real game, with better rendering on top.

**Play in your browser: <https://opentennis.dev>** (you pick your own ISO; nothing is uploaded).

Newest unreleased build (every change to main, less tested): <https://beta.opentennis.dev>.

**Download** for Windows, Linux, Android or iOS: [latest release](https://github.com/FIZL0/opentennis/releases/latest).

Work in progress. Building from source, modding by hand and development: [docs/DEVELOPING.md](docs/DEVELOPING.md).

## Screenshots

Taken from the remaster running on a retail disc (the art comes from the disc; none of it is in this repository).

| | |
|---|---|
| ![A rally on a grass court](docs/screenshots/rally-grass.jpg) | ![A rally on an autumn court](docs/screenshots/rally-autumn.jpg) |
| ![Serving](docs/screenshots/serve.jpg) | ![Main menu](docs/screenshots/menu-main.jpg) |
| ![Character selection with mod characters](docs/screenshots/menu-chars.jpg) | ![Court selection](docs/screenshots/menu-courts.jpg) |
| ![Umpire selection](docs/screenshots/menu-umpires.jpg) | ![Online lobby](docs/screenshots/menu-lobby.jpg) |
| ![Mod store](docs/screenshots/menu-browsemods.jpg) | |

## You must provide the game disc

**This repository contains no game data**: no models, textures, animations, sounds, music or program code from the
original. Everything the game shows or plays is read **at runtime from your own copy of the disc**. Without it,
nothing runs.

What you need:

| | |
|---|---|
| **Disc** | Hot Shots Tennis, **US release, version 1.00, serial SCUS-97610** (PS2) |
| **Format** | A disc image (`.iso`) you made from **a disc you own** |
| **Where** | Anywhere; the game asks for it the first time (the browser keeps it in private storage, nothing is uploaded) |

- **Other regions don't work.** Everybody's Tennis (Europe), Minna no Tennis (Japan) and other versions have
  different program data. The game checks the disc (`SYSTEM.CNF` boot file `SCUS_976.10` and the size and header of
  `ZZBIN/GAME.BIN`) and refuses anything else with `unsupported disc`.
- **Don't ask for, share or commit disc images or anything extracted from them.** Nobody here can provide one.
- **You don't need a PS2 BIOS, an emulator or any other files** to play. Those are only for the research tools
  (see [docs/DEVELOPING.md](docs/DEVELOPING.md)).

## Controls

Player 1 uses the keyboard or the first controller. In doubles, a second controller plays player 1's partner (P1 + P2
vs 2 CPUs); in singles it plays the opponent. Every other player is a CPU.

| Action | Keyboard | Controller |
|---|---|---|
| Move (aim while swinging) | WASD | Left stick / d-pad |
| Topspin | J | A |
| Slice / Serve | K | B |
| Lob | L | Y |
| Drive | U | RB |
| Swap hand (character select) | C | Select |

## Mods

Main menu → Mods adds characters, courts and umpires from inside the game, on desktop and in the browser. The store
is the Hugging Face repo [`FIZL0/opentennis-mods`](https://huggingface.co/datasets/FIZL0/opentennis-mods).

- **Add mod** installs a mod `.zip` from your computer (the system file picker).
- **Browse mods** lists the store's mods, each with its picture, stats card, author, handedness, costumes and size,
  tagged *Verified* (merged), *Not reviewed* (an open upload), *Installed* or *Update*. Press one to install it; it
  appears in the character select when done. *Refresh* fetches the list again.
- **Upload my mod** sends one of your installed mods to the store as a pull request. Sign in once with your own
  Hugging Face *Write* token: *Sign in* → *Open token page*, copy the token, then *Paste token* (the game reads
  it from the clipboard; in the browser, paste it into the page with Ctrl+V first). *Signed in* → *Sign out*
  signs out. Then a mod → *Upload*
  uploads it: the game packs it, adds its preview and stats, and reports the pull request's URL. Other players see
  it in Browse straight away as *Not reviewed* until it is merged.

Upscaled textures: a PCSX2 texture pack made for the original game works as is, and mods can carry upscaled
textures of their own (the "upscaled textures" setting). How to make either: [docs/DEVELOPING.md](docs/DEVELOPING.md).

## Online play

Main menu → Online: host a lobby, find lobbies, quick play, or join with a code (paste it, or scan the host's QR).
Find lists every open lobby (singles and doubles, the host, court and open seats); you pick your characters after
joining, in the lobby, which holds your seats meanwhile (Characters picks again, the host's Court changes the venue).
Set your name under Online → Name; the lobby shows it on your seats. Guests press Ready, and the host presses Start
when everyone is ready. Any seats still open are played by the computer. The host can also take a guest out of the
lobby (OK on their seat).

In the browser, join by code
(paste the code on the page with Ctrl+V, or open the page with `?code=<code>`) or host; Find isn't there, and Quick play
hosts. Browser and desktop players can play together.

Browsers on one network can also play with no internet and no server (WebRTC): the host sets the relay to Off and
hosts; each player scans the host's QR with their phone camera (it opens the game with the code), joins, and shows a
QR of their own, which the host scans in turn (the tab it opens hands the code to the game tab), or the codes are
pasted both ways. Browser-only: desktop builds can't join these lobbies.

LAN play needs no internet: put everyone on one network (a router, or one player's hotspot with mobile data off) and
set the relay to Off. Find lists the lobbies on that network first (mDNS), and a host's code holds its LAN address,
so joining by code works there too. Your firewall must let mDNS (UDP 5353) and the game's UDP port through.

Relay and name settings: Online → Relay / Name (details in [docs/DEVELOPING.md](docs/DEVELOPING.md)).

## Legal

This is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or Clap
Hanz. Hot Shots Tennis, Everybody's Tennis and Minna no Tennis are trademarks of their owners. No copyrighted game
data is included or distributed. You must own the game to use this.
