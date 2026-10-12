# M2i — mods in online matches

## What's in
- **Gameplay always travels.** A lobby pick of a mod (`net::Pick.mod`) carries `mods_online::Wire`:
  - donor, hand, TParam base row with its overrides, and the AI row;
  - the picked costume's skeleton, with the rest matrices as f32 bits so JSON can't round them;
  - the hash of the mod's looks (its pack).
  Every peer, the owner included, builds the match's `MatchMod` from the wire (`for_match`). It then re-derives the
  sim's motions/paths/pelvis/arm/stance ball on the wire skeleton (`play/mod_match.rs::load`). The drawn clips stay
  in `CharacterData.visual`. So a peer that declined the looks simulates the same match: the handshake's
  `gameplay_hash` matches, and the bodies, tosses and contacts match too.
- **Why the skeleton has to travel.** The sim reads `CharacterData.skeleton` and the motions retargeted onto it. This
  covers serve `toss_setup`, the stance ball, `bodyhit` and the cutaway bones. A mod's glb skeleton differs from the
  donor's disc skeleton, so a stand-in posed on the donor's skeleton would drift.
- **Checks before use.**
  - The host refuses a Hello whose wire fails `Wire::check`: ranges as `mods::read`, real TParam columns, numeric
    values, a parent-before-child tree of ≤ 512 joints.
  - A guest leaves a room whose wires fail it.
  - The lobby message cap went from 64 KB to 1 MB, because a skeleton is ~30 KB.
- **Looks are optional, peer to peer.**
  - On a Room, the menu settles each mod (`Menu::settle`). If it's here already, it uses that copy: a roster mod with
    the same id and pack hash, or `mods-online/<hash>/` beside the disc. Otherwise it follows the remembered answer
    (`mods-online/ask.txt`: always / never / ask). With no answer, it asks: *Download, always · Download this one ·
    Stand-in this time · Never download*.
  - A download is `Cmd::Want(hash)`. The host serves its own mods, passes on (and keeps) packs from the owner guest,
    and asks the owner only once. Packs go on QUIC uni streams.
  - A pack holds only `glb/png/wav/json` files. `textures/upscaled/` is left out. Packs are capped at 64 MB.
  - A received pack is checked for its hash, safe relative paths, size and file count. It is then written,
    `mods::read`, and §7 `conform` for every costume (`mods::check`). Any failure deletes it and plays a stand-in.
- **Stand-in** (declined or refused): the donor's disc costume (costume 0 if that number doesn't exist). Every texture
  of its parts, racket and GS materials has its RGB channels rotated, a 120° hue turn. It plays no mod voice.
- **Ready gate.** When the room has mods, the host waits for `Ready(ver)` from every guest and from itself before the
  start's round trip. `Room.ver` counts line-up changes, so a stale Ready doesn't count. Rooms without mods start
  exactly as before.
- The lobby shows a mod seat by its name. `HST_MENU=lobby-ask` gives a shot of the prompt.

## Checks
- `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`: a host and a guest on loopback. It checks that:
  - the wire arrives without the owner's folder;
  - Want returns exactly the folder's pack;
  - no Start comes while only the guest is ready;
  - Start comes once both are.
- `mods_online::tests`:
  - the wire round-trips bit-exact, and bad wires are refused;
  - pack/unpack keeps only data files, and refuses a changed byte, `../`, a cut-off pack, or a non-mod (nothing is
    left in the cache);
  - the choice is remembered.
- `play::mod_match::tests::stand_in_simulates_like_the_mod` (needs the disc and `context/mods/test_pc00`): the mod
  with its looks and a stand-in from the same wire hand the game an identical skeleton, pelvis and every motion's
  locals at 8 times. The stand-in draws the donor's own rig.
- `tools/check.sh -p hst`: 130 passed.
- A shot of the prompt at 1280×720 (`context/m2i/ask2-16x9.png`).

## Not verified / not 1:1
- **Two real machines, and a real mod moving over the internet.** Only loopback was tested, with a toy pack. Big
  packs (11–17 MB Fore mods) over a relay are untested for speed. The download shows no progress, only the prompt
  closing and the match waiting.
- **The stand-in's colours, by eye in a match.** No online match could be launched in this unattended run, so this
  wasn't seen. The format check (`Rgba8Unorm[Srgb]`, which disc textures use) and the code are all there is.
- **The prompt at other aspect ratios.** Hyprland's socket wasn't reachable from this run, so `tools/shot.sh` couldn't
  size a window. Only the 16:9 shot exists. The prompt uses the Lobby's own 640-space helpers (`rows`, `text`,
  `info`), which M2f laid out for any aspect.
- **Courts.** There are no court mods yet. The fallback rule ("a peer without the court mod plays its disc layout's
  court") was added to M8f, where court mods arrive.
- **Not checked against the original.** Nothing here exists in it. The menu is ours to design (PLAN), and the
  gameplay data is the existing mod path's, which is unchanged.
- A peer that never answers the prompt holds the start forever. Leaving is the way out. There is no timeout.
