# Reference

## Controlling the real game (for recordings and checks)

- `tools/pcsx2-hst.sh` launches PCSX2 with PINE on; `tools/pine.py` reads/writes RAM and loads/saves states.
- What you can do with your PCSX2: `tools/pcsx2-hst.sh` start · `stop` · `status` (is it up); `tools/pine.py` (no args)
  prints the game id and running/paused, `tools/pine.py <hexaddr>...` reads RAM; pine tools resume a paused copy
  themselves; `tools/screenshot.sh out.png` takes a screenshot (a copy's own F8 hotkey); `tools/vpad.py` presses
  buttons. pcsx2-hst.sh takes no other words (they'd be opened as a file and block the copy with an error dialog).
- **Matches end.** A match finishes on "Game, Set, Match!" with "✕ Continue" and stays there; nothing after it is
  gameplay. Long captures (slot 5 runs a whole bot match) can run past it: before trusting a long capture, take a
  screenshot at its end, and cut or re-record what came after the match ended.
- **Save states (the user's — load only, scratch saves go to 8/9):** 3 = start of a game, P1 human + 3 bots;
  4 = mid-rally right after the serve; 5 = full bot game. Slot 5 has no human input — use it for ball/AI
  captures only. Controller-input recordings (P0 and anything gameplay-from-input) start from slot 3 or 4 with
  P1 driven by `tools/vpad.py`.
- `tools/vpad.py serve` creates a virtual Xbox-360 pad (uinput, no root); PCSX2 binds it as `SDL-0` when no
  real controller is connected. With `HST_PCSX2=N` (parallel runs) it is pad N, the only pad PCSX2 copy N sees. `tools/vpad.py send "press cross 120" "stick l -1 0" "sleep 300" release`.
  Never press Select (PCSX2 hotkeys are Select + shoulder combos). Timing is wall-clock; frame-exact replays
  should use PCSX2 input recording (`.p2m2`) — P0 decides.
- `tools/screenshot.sh out.png [pattern]` captures a window (default PCSX2) without focusing it; with `HST_PCSX2=N` it uses
  copy N's own F8 screenshot instead (copies sit on hidden workspaces, where grim sees nothing; full-size PNG; not while paused).
- Remaster shots: `tools/shot.sh out.png [hst args…]` runs `hst … --shot` in a **floating 1280×720 window on a hidden
  special workspace** (Hyprland `hl.dsp.exec_cmd(cmd, { float = true, size = '1280 720', no_initial_focus = true,
  workspace = 'special:hstshot silent' })`), so every shot is the same size (2048×1152 at the desktop's scale) however
  many windows other agents have tiled; launch any window you compare across runs this way. `HST_ISO=` a symlink in a
  folder without `mods/texture-replacements` shows the disc's own textures.
- `vpad.py send` returns at once and the server runs commands one after another (sleeps included): a script
  that sends several batches must wait each one out (`vpad()` in `tools/pick.py`), or presses pile up and land late.
  In menus ○ confirms, ✕ goes back.
- **Choosing characters:** `tools/pcsx2.sh python3 tools/pick.py <char 0–13> [slot]` loads save slot 2 (doubles
  character select), puts character <char> in as P1, takes the three COM picks where their cursors start, starts
  the match and saves P1's first serve to the scratch slot (8). It prints P1's character (+0x12bc), hand
  (+0x12b4) and model hand flag, and stops if the wrong character got picked. All 14 checked (P7b). Slot 2's P1 has
  the "switch hand" toggle on, so every pick plays with the hand opposite to TParam's. To serve from that slot,
  press ○ three times about 1.5 s apart: the first press after a load or a point is ignored, the second tosses,
  the third hits. P1 serves every point there and points are short (about 2 s).
  Not automated yet: choosing the COM characters (each COM cursor starts on a different character), the costume
  (L1), the switch-hand toggle, singles, P1 receiving.
- Verify input effects numerically over PINE (e.g. ball/player state), not by eye.
- `tools/single.sh` (in tmux) runs `claude continue` (TUI: attach to watch or type; it never asks) back to back and owns the virtual pad for the night.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```

Without the ISO (M13): `hst --import "Hot Shots Tennis (USA).iso"` once (or plain `hst`, which asks for the ISO in a
file dialog on first start) copies the 171 MB the remaster reads into `~/.local/share/hst/disc.pack`
(`%APPDATA%\hst` on Windows); after that `hst [flags]` runs from the pack (settings.txt, controls.txt, mods/ beside it)
and the ISO can go. `hst <iso> …` still runs straight from an ISO.

In the browser (M13a, M3): `tools/web.sh` builds `web/pkg` (WebGL2) and `web/pkg-webgpu` (the page takes WebGPU when
the browser has an adapter; `?webgl2` forces WebGL2); serve `web/` on localhost or https (`python3 -m http.server -d
web 8765`; GitHub Pages: `.github/workflows/pages.yml`). The first visit asks for the ISO and imports the pack into the
browser's storage (OPFS); later visits run from it, menus and all. Flags go in the URL (`?play&singles&stage=1` =
`--play --singles --stage 1`); `?forget` drops the pack. Sound starts on the first click/key. Online: relay only (no
DHT/mDNS), host or join by code (`?code=<code>` or Ctrl+V on the page, then Join by Code); plays with desktop peers.
Short codes (B104): a hosted lobby shows a 4-character code (lobby screen, QR = its link); Join by Code types it (pad:
on-screen picker, keyboard: type, phone: OS keyboard) or takes a pasted code/link; "Copy Link" (OK on the lobby) gives
`https://opentennis.dev/?code=<code>`, which opens straight into that lobby's character pick
(desktop: `HST_CODE=<code> hst`). CLI: `hst net host` prints `Short("…")`, `hst net join <short>`.
Chat (M20): in an online lobby, character select or match, Enter (or tapping "Chat") opens a line, Enter sends, Esc closes; pads: R2 steps preset phrases (and Mute/Unmute per sender), L2 says it; `/mute NAME` `/unmute NAME`; Settings → Mods → Chat off hides it. Through the host, outside the input stream.
Mods in the browser (M16): main menu → Mods → Add mod picks a mod `.zip` (checked whole by `modfs::install`, kept in
OPFS `mods/<id>/`, loaded into memory at start); ✕ on a mod → Remove (B202's child buttons beside the row; ○ or a tap elsewhere closes them) removes it; Reload brings changes into the select.
LAN between browsers (M2k, `net/rtc.rs`): relay Off → Host gives a WebRTC offer QR (`?code=rtco…`), the guest's answer
QR (`?code=rtca…`) goes back to the host's tab (a tab opened with it hands it over by BroadcastChannel, or paste).
Two-browser check: `tools/web_lan.mjs` (host, guest and answer pages in one headless Chromium, RTC stats, hashes).
Headless check: `tools/web_check.mjs` (`WEB_INPUT` key/click script, `PROFILE`, `CHROME_FLAGS="--enable-features=Vulkan
--use-angle=vulkan"` for the real GPU; SwiftShader's WebGPU loses its device). M9: installable (manifest, "Add to Home Screen"; fullscreen landscape), plays offline once loaded
(`web/sw.js`), on-screen touch controls on touch screens (`web/touch.js`: a virtual standard gamepad, hidden while a
real pad is connected; `?touch` forces them on a desktop), settings/controls in localStorage, phones start on the Low
graphics preset. Touch check: `tools/web_touch.mjs` (phone viewport, taps, offline).

Doubles by default (`--singles` for 1v1). Player 1 = keyboard + controller 1, player 2 (player 1's partner; the opponent in singles) = controller 2
when connected, players 3 and 4 (the other team) = controllers 3 and 4 when connected; the other slots are CPU. `--pads a,b,c,d` seats
1P..4P instead (`k` keyboard, a pad index, `-` CPU; the main menu passes it), e.g. `--pads -,k` = CPU 1P, keyboard 2P. WASD/left stick/d-pad move (and aim at contact, screen-relative) ·
J/A (✕) topspin · K/B (○) slice · L/Y (△) lob — stick toward the net at contact: flat; pulled back with slice: drop · any shot button serves (as its toss: ✕ strong, ○ weak, △ underhand) · Space/X (□) replay after a point ·
C/Select swap hand at character select (no free camera, B218).
Rebind any of these in `controls.txt` beside the ISO (written with the defaults on first run). Controllers can be
plugged and unplugged mid-match: each keeps its slot, a new one takes the first free slot. **Steam / Big Picture**: add the
installer's launcher as a non-Steam game; Big Picture and the Deck start it full screen (`--fullscreen` anywhere else), and under
Steam Input only Steam's virtual pad is read (the physical pads Steam lists in `SDL_GAMECONTROLLER_IGNORE_DEVICES` are
skipped), so Steam's per-game layouts apply; the whole menu flow, including Quit, works from the pad. Launch options for X11 (Steam overlay on a Wayland desktop): README *Steam*. Wide windows show more court at the sides (the game's vertical view kept); the HUD stays on a centred 4:3 screen.
`HST_AUTOPLAY=1` makes every slot CPU (unattended tests). `--umpire-mod DIR` draws an umpire mod's `umpire.glb` on the chair (M8f4; timing/calls its donor's). `--stage 01..11` court, `--court 0..11` surface. Drawing runs uncapped (the simulation stays a fixed 60 Hz tick, visuals blend the last two ticks); `--vsync` caps it to the display.
Character select previews: cached PNGs in `cache/previews/<boot file>-r<N>/` beside the ISO (drawn on first run, disc characters first; mods may ship `previews/<costume>.png`), the live model after 0.3 s on a card or on R2/L2/△. `HST_PREVIEW=png|live` forces one, `HST_BAKE=all` (with `HST_MENU=chars`) draws every mod costume and quits, `tools/select_stress.sh SECS DIR` scrolls the select at full speed under a VRAM watchdog.

Character select (B368, `play/main_menu/roster.rs`): one column per player (2 singles, 4 doubles), each with its seat's colours, header (seat, character, device), costume / hand pills, preview beside the play style and stats card, and its own face grid that scrolls on its own (keys/pad, the wheel or a drag over that column); △ does nothing. `HST_MENU=roster1..roster4` shoot 1–4 humans, `roster2-deep` 2P scrolled into the mods.
`--mod DIR [--mod-slot N]` puts a standard character mod (README *Character mods*) in player N (default the first), costume by `--outfits`.
Gamepads whose device node is read-only (udev rules that strip write to stop rumble) work through the patched
`third_party/gilrs-core` (read-only fallback, no rumble).

Menu art (B136): the menus' disc sheets are rebuilt at 4× with their labels set as vector text (`play/menu_art.rs`); `--original` keeps the disc's sheets, `HST_MENU_ART=DIR` dumps the rebuilt PNGs. B141: title bars, buttons, mode bars, character cards/tags/rank pills, group boxes, confirm labels and the pause dialog are our own vector pills and boxes (`menu_art::pill_layers`, `pill_quads`/`box_quads`) with our words and colours (`main_menu.rs` `TITLES`, `MODE_FILL`, `CARD_FILL`, …), with or without `--original`.

Player names (B371): on the select △ opens the player's name picker (saved names, New name… with an on-screen keyboard or the keyboard, No name; □ forgets/deletes); names live in `names.txt` beside the settings. A named player's name replaces 1P–4P on the select card, over the player before the serve (`play/name_tags.rs`, a UI tag in the panel colour instead of the marker), on the point boards' and final board's plates and the stats page; online each player's name travels in its `Live` pick. `--names a,,b` (match order) for direct matches; `HST_MENU=names|names-type` for shots.
Self-updater (B374): a release build (`tools/release.sh`, built with `HST_VERSION=<tag>`) checks GitHub's latest `v*` release at start; the main menu's NEW VERSION OUT card offers Update (✕: download the bare binary beside the game as `hst.new`, check minisign-signed `SHA256SUMS`, swap, restart with the same args; Windows leaves `hst.exe.old`, removed next start) or Later (○, this run). Unwritable folder / Flatpak / Steam: Download (release page). Off: Settings → Online & Mods → Check for Updates, or `--no-update-check`. Test hooks: `HST_UPDATE_API=<json url>`, `HST_UPDATE_NOW=1`.

Button legend (M14): every menu, the pause menu and the result screen list their actions with our own glyphs along the bottom strip (`play/main_menu/legend.rs`), for the device last used (keyboard keys as bound, Xbox, PlayStation by vendor, Steam Deck), per player on the select and seat assignment; a touch hides it. `HST_LEGEND=keys|xbox|ps|deck` forces a style for `--shot`s; the test `legend_lists_what_each_screen_handles` probes `Menu::step` for each screen's presses.

## Done (ported and verified against the original)

- B345: characters, shade copies and effects are frustum-culled with skinned/morphed bounds (`cull.rs`; Settings → Graphics → Skip Off-Screen, `--cull off`, `?cull=off`; `HST_CULL_CHECK=1` proves the boxes); native bench faster, browser unchanged; occlusion culling tried and dropped (journal 2026-10-14-b328-mobile-fps/3-CULL-FINAL).
- B351: Graphics preset Phone (100 % scale, shadows Low) picked on a touch screen's first run; at Shadows Low only the players, the umpire and props whose shadow reaches the court cast (`shadow::Minor`, `reaches_court`; journal 2026-10-14-b351-phone-defaults).
- B364: Settings → Graphics → Mesh (100/75/50/25 %, `--mesh N`, settings.txt `mesh = N`) draws characters, their rackets and the court figures with fewer triangles (quadric edge collapse of the index buffer only, `crates/hst/src/lod.rs`); 100 % leaves meshes untouched. Takes effect for characters loaded after the change. Journal 2026-10-14-b364-mesh-detail.
- B343: Settings → Display switches Fewer Shadows (on: B351's Low caster cut; off: Low casts like High) and Batch Crowd (off by default: a court's identical creatures share draws, light scale in their `MeshTag`; no measurable gain); `--fewer-shadows on|off`, `--batch-crowd on|off`, URL `?fewer-shadows=off`. `tools/bench.sh` now draws the court (`--stage 1`; before it drew none). `HST_FPS=1` also logs draws (meshes seen, mesh/material pairs, shadow casters per cascade). Journal 2026-10-14-b328-mobile-fps/3.
- B344: character sway works out its noise once per tick and per distinct entry (half the time, same output); Settings → Display → Sway LOD (`--sway-lod on`, `?sway_lod=on`) updates far players every other frame and unseen ones not at all, off by default (no fps gain measurable; journal 2026-10-14-b344-sway-lod).
- B24a: court 4's passing ball (trigger type 15): rolled once a match at a change of ends, route, animation, bounce sounds and fade bit-exact against a recording (route 0); drawn in play (journal 2026-10-08-b24-court-events).
- B31: the 3D scene blends on gamma-encoded values as the GS (HDR scene cameras into a raw target, gs.wgsl and Bevy's PBR output encoded; hud_gamma.rs); court `@add` lines now add the same to each channel (journal 2026-10-08-p17q-tornado/3).
- P2d: the rally aim returns its nudge (±5/±10), short-only flag and the smash's held depth (`shot::Aim`) into the launch; the smash launch gets the original's timing scatter (`swing::smash_scatter`/`smash_scale`); aim.rs checks all three per recorded aim and that ×0.6 would miss a sweet incoming slice in singles and doubles, timing.rs 28 smash launches bit for bit (journal 2026-10-08-p2d-aim-leftovers).
- B21: timing balloons sit just above the head (neck anchor) at the original's size and timing; singles players show the swirl on losing a point.
- P22: widescreen HUD on a centred 4:3 screen (play/widescreen.rs; the 3D view was already Hor+, F0 already uncapped), rebindable keys/pad buttons in `controls.txt` and hot-plug-stable controller slots (play/controls.rs); render/input only, hst-sim untouched, full test suite passes.
- B117: Aspect Ratio setting (Auto, 4:3, 16:9, 16:10, 21:9; replaces Widescreen; phones default 16:9): a fixed ratio draws everything into the window's centred rect of that ratio, black bars around (`widescreen::view`, `hud_gamma` composite viewport); `--aspect 16:9` on the command line, `--4x3` = 4:3.
- B7: one-sided court materials back-face culled as VU1 does (MDL material +0x24 = 0; winding from the third vertex's UV w); court 1's near school fence no longer blocks the camera (mdl.rs, gs.rs; tests/winding.rs; journal 2026-10-07-b7-camera-blocked).
- P18: upscaled and moddable textures — PCSX2 pack hash names computed from the disc GS data (XXH3 of swizzled blocks + CLUT), lookup mods/texture-replacements/ (key-named files → PCSX2 pack) → disc, `--dump-textures`, hot reload with HST_TEXTURE_RELOAD=1 (B83; hst-data texhash.rs, hst textures.rs; journal 2026-10-07-p18-textures/README.md).
- P17k: weather exact — schedule from the MT19937 seeded by a newlib `rand()` output (bit-exact vs slot 5), mirrored ripple tiles, rain voices sweeping ±45°, costume `.NOI` wind sway (EE phase bit-exact, VU noise from the microcode), particles under the court fog (journal 2026-10-07-p17-court-rendering/8-WEATHER-EXACT-FINAL.md).
- P17m: costume sway exact — `.NOI` entries moved in their bones' spaces (posed), deformers restarted on every motion set; VU noise matches a GS-dumped frame of pc00's skirt and fringe to 0.04 px (journal 2026-10-07-p17-court-rendering/10-SWAY-EXACT-FINAL.md).
- P17h: court model mipmaps — TEX1 MXL from MTL header +0xf, K from MDL material header +4, level ⌊log2(view depth)+K+½⌋ (journal 2026-10-07-p17-court-rendering/6-MIPMAPS-FINAL.md).
- P7c: reach and contact heights — `hst_sim::player::ReachStats::from_tparam` (the game's strtok/decimal parser) bit-exact against RAM records for all 14 characters (tests/player.rs reach_stats_from_tparam); play.rs gives each player its own contact-search reach.
- P17g: court shadows — sun direction bit-close to RAM on courts 04/10, strength 1 − (⌊0.xx·255⌋·255>>8)/128 multiplied on the hole ground (measured on the PS2 shot), casters = players + plant records with code byte 3 ≠ `'0'` (shadow.rs, gs.rs, gs_prepass.wgsl).
- P17v7: net shadow — the net flattened along the sun onto the ground as a black decal at the shadow strength (weather-scaled), since the shadow map loses its thin strands; not 1:1 (user 2026-10-12) (main.rs, weather.rs).
- P12b1: point score pop-up — player plates, score roll (old value squashed, new grows) + white flash, Deuce! banner with × count, from flow's score show; layout checked on screen against the original (play/popups.rs).
- P11e2: singles centre walk + net dash — `hst_sim::position::{zone, Single, Court}`, `play.rs` ai_wait_singles; test tests/singles.rs on context/fixtures/ai_pos_singles.bin.
- P11e1: doubles formation — each bot's lane, front flag and waiting spot (point start, re-pick on hits / every 30 frames, hold) and the centre-rate/radius walk back, exact on every change in ai_pos_s05 (position.rs, play.rs ai_wait).
- N4: landing markers — the game's `chakudan_p` (red, at the aim, from the serve return) and `smash_p` (yellow, smash-point search) models; red checked on screen against the original; N4a: yellow's placement frame and point bit-exact on a forced-lob rally (no path growth on the launch frame), fade-in and look match on screen (effects.rs, play.rs). B91: marker drawn after the court lines, layers bottom-up (no sort flicker).
- N5a: △ smash (kind 1) — contact search, launch (smsh1, 5° spin) and flight to the first bounce exact on lob_smash_s05; one locked swing per team (swing.rs, shot_tables.rs).
- B57: CPUs let out balls go — the predicted path's line call (mode) and its distance past the nearest line (gap, kept stale as the original's) feed the NET/BASE routines' let-go hold (out by ≥ the row's line margin → stand); hold replayed bit-exact on ai_let_go_s05 (lines and margins poked) (play/doubles_ai.rs).
- N3a: sound banks (`hst_data::snd`), note → tone and SPU pitch bit-exact vs save states (sound.rs).
- Disc/XB/TIM2/MDL/MTL readers; court layout placement; Bevy renderer, 60 Hz fixed sim + interpolation.
- Ball flight (drag, Magnus, gravity, curve/bend), ground bounces, rolling — 4524 recorded frames.
- Shot tables (TRAJ): lookup + launch speed/elevation/frames — 11 recorded strokes.
- Per-character shot parameter records (17 per class/kind, record = character + 3) built from GAME.BIN —
  bit-identical to the game's runtime table, using the PS2 FPU model.
- `hst_sim::ps2` — PCSX2's EE FPU model: chop rounding, add/sub alignment with one guard bit, **div and sqrt
  round to nearest** (the emulator's divider mode), DAZ. Proven on 2912 table values and the ball integrator.
- Ball flight **and bounces** on PS2 arithmetic in the original's instruction order: all 4524 frames of 39
  recorded shots (flight, curve/bend, every bounce, rolling) bit-exact in position and velocity. Pieces:
  `hst_sim::vu0` (VU0 chop model), `contact` (plane sweep, contact point — the original lerps to the raw hit
  fraction; its 0.005·r back-off only gates the ≤ 0 test), `quat` (matrix↔quat, VU0 slerp microprogram),
  `libm::sinf` (the game's fdlibm sinf, incl. its pio2_2 = 0x373543ff).
- Net contact (flat net from the game's predictor), material-based bounce response.
- Collision world placement: props from plant records (VU0 sin/cos, rotations, inverse) and the 20 m prop grid —
  117 props and every grid cell bit-exact on court 10.
- Live ball against the world mesh (court model, walls, net, props via the 20 m grid), ground material from attribute
  maps, material table response — 12498 recorded frames of a bot match bit-exact, all contacts included.
- First-bounce turn of serves (`+0x1b0`, game's atan2f) — round1's turned serves bit-exact; `--stage` play on
  the world mesh.
- Live-ball point verdicts (`hst_sim::judge::Rally`) — every decision of a recorded bot match frame-exact, rally
  block equal every frame (faults and net points included).
- Post-point scoreboard timeline (`hst_sim::flow`): pause, wait, score shows, change ends — every call-free
- Post-point umpire call show (settle on voice end/countdown, hold, fade; out/double fault chain into the score show), human press ends the phase once the score settles, HUD old score until the roll — all 35 point-over phases of a recorded bot match and 8 human phases tick-exact (P12a).
  point-over phase of a recorded bot match tick-exact.
- Stroke contact search + timing grades (SWEET SPOT / QUICK / SLOW) — ground-stroke branch.
- F0 — uncapped frame rate: render-side blending of positions, facing, motion time, camera and balloon fades; sim unchanged.
- N1d1 exact ANI motion sampler (squad/Hermite, bone-length scale), bit-exact vs RAM.
- Motion clock (N1d2): per-frame motion time as the game's motion player — sample at the wrapped/clamped time, then add the speed; soft follow-through held by its 8-frame crossfade — bit-exact on 35026 recorded ticks (anim_s05.bin).
- Court collision mesh in play (P15): the ball always meets the disc court's mesh (net, cord, posts, walls, props);
  aimed live-ball recordings (post hits, cord dribble-overs in rallies and serve lets, cord/net stops) bit-exact.
- Shot buttons (P5a): ✕ topspin, ○ slice, △ lob only; the stick at contact turns topspin flat (within 60° of forward) and slice into a drop shot (within 45° of back; not on serves; smashes unchanged) — 42/42 recorded human aims in round1.bin.
- P14c2: ambient sound emitters (`npc::Emitter`: countdown, re-arm, pan bit, type-36 sweep) bit-exact on courts 10, 1, 2, 4; wired into play's court sounds.
- P14d: court 5's stand crowds (roster 61..66, `npc::Crowd`: atlas column, cheer level ping-pong, stand up/seat per match phase) bit-exact against a court 5 recording; drawn as the `wim-0N` billboards with their UV atlas frame.
- P14f: trigger creatures walk their disc paths (category-14 markers sharing the linked one's sub byte, file order; node radius at 0x30), matching the path manager in RAM on courts 1, 2, 4, 10; rows without `exact` aim at a random point on the node's circle (x/z bit-exact against the reset targets in RAM); the app draws every creature where its tick puts it (court 1's chickens run off when startled).
- B74 court backdrops: sky/clo/acc/bg follow the camera eye + envir lift, rotated by the envir yaw − hole yaw (row 0); matches RAM on courts 1, 2, 4, 10 and a court-10 screenshot overlay (`HST_EYE` fixed-view shots).
- B80 court model draw: MODULATE materials blend (ABE), `@add` drops fog, the hole model drawn faded (MODULATE, A ≥ 0x80 test, vertex alpha) and first in material order, MODULATE alpha on GS integers; load rules match RAM on all 11 courts (8,821 batches), grass vs screenshots on courts 3 and 9.
- P3f5 lens flare: the sun's 24 rays (rand() brightness/angle jitter in `Rngs::flare`), its ghost sprites (`flare3.bmp`) and the whiteout drawn on the match camera; matches a poked-sun PCSX2 shot by eye (no occlusion fade).

## Known gaps / caveats

- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only); live-ball
  recordings do (`tests/live.rs`). The app without `--stage` collides with court 10's mesh (court drawn flat).
- `libm::sinf` ports only |x| ≤ 2^7·π/2 (asserts beyond); the game's callers stay in [0, π].
- Disc court folder = RAM court index (0x422f90), checked for 7 (indoor gym) and 10 (ruins, slot 5) in B92. Court
  carousel order from the court-select start (s2's slot 08 save): steps right 0…10 → courts 5 1 4 6 2 8 9 11 10 3 7.
