# B40b — 3D models in the character select (inspect pose)

## What was built

- `character.rs`: `disc_motions` also loads the menu inspect pose from `MENU/PC/PCnn.XB` (`pcNN_pose.ANI` as a Clip, `pcNN_pose.MOR` as a face) under the key `character::POSE` (0x40). `pose_frame(n, length)` gives the frame the original holds. Mods call `disc_motions` with their donor, so they get the donor's pose with no mods.rs change. Test `inspect_poses_bind_and_hold_the_games_frames`.
- `inspect.rs` (new): `inspect::spawn(commands, data, n, hand, layer, rect)` puts one character, posed, lit and framed like the original's inspect screen, into a screen rect on its own camera and render layer. `inspect::apply` plays R2 gu_set (motion 0x2e) / L2 di_set (0x2f) / △ back to the pose. The `--inspect all|N[:C],…|MODDIR,…` viewer shows a row of them. This is what B40's select will call per slot.
- `shade.rs`: a `FixedLight` component lights a rig with a fixed light (no court/weather).

## Findings in the original (PCSX2)

- The pose is held static: the motion clock never advances on the inspect screen. Start frame (the screen's pose-start function at 0x3236b0): if a per-character flag is set (flags by char 0..15 = 1,1,0,1,0,1,0,1,0,0,1,1,0,1,0,0), frame = (char 0 ? 70 : int(anim length)) − 1, else 0. The object's byte +0x7cf holds the character; anim length at +0x2c equals the port's `Clip.length`. Live held frames: c0 69, c1 61, c2 0, c3 62, c4 0, c5 66, c6 0, c7 57, c12 0, c13 64. The inspect grid is not in character order (0,1,2,5,13,4,3,6,7,12,…).
- Buttons: R2 = gu_set (0x2e), L2 = di_set (0x2f), each at speed 1 from frame 0 to its end, then held. △ = back to the held pose at yaw π. L1/R1 change costume. The yaw is set instantly; the loss (L2) yaw is 2.443461 for char 4, 2.356194 for char 9, else π. Sign checked from the world matrix (row0 = cos θ, 0, −sin θ).
- Light (RAM light matrices): two lights, colour 0.4 each, directions (0.7071,0,0.7071) and (−0.819152,0,0.573576), ambient 0.47. A third light (0.383,0,0.924) has zero colour.
- Camera: focal 1194.256 px horizontal (30° over 640) and 1114.64 vertical (448-line frame). The 2.79904 seen in RAM is PCSX2's widescreen patch (×0.75). Axis at pixel (520,176) of 640×448. Model at game (0,1.5,8), yaw π, mirrored in x for left-handers (TParam hand). Near 0.01, far 3072.
- Bevy pitfall: a second camera on the same target is alpha-blended by the upscaling pass, and gs.wgsl writes specular into alpha, so the model looked translucent. Fixed with `CameraOutputMode::Write { blend_state: Some(BlendState::REPLACE), .. }`.

## Verification

`--inspect N` shots of all 14 characters side by side with the original's inspect screen (tools/screenshot.sh); images in context/b40b/side0.png and side1.png (git-ignored). Every pose, facing, racket hand and framing matches; lighting is close by eye. Mods: `--inspect` with three Fore! mods (phoebe/pc00, brad/pc05, falcon/pc09 from ../HST-MODS/out/mods) shows each in its donor's pose (context/b40b/mods.png).

User note (2026-10-08): the in-game Data menu's Costumes tab shows each character's costumes separately, and the Umpires tab has the umpire 3D models — sources for costume previews and umpire-select previews (B40b4).

## Not verified / not 1:1

- The select itself: B40's character select (play/main_menu.rs) doesn't exist yet, so the done criterion (a `--shot` of the select with every slot hovered) can't be met. The previews run in the `--inspect` viewer only → B40b1.
- UVA eye UV animation (`pcNN_pose.UVA`): done in B40b2 (below).
- Chars 5 and 13 draw gu/di through another path in the original (object +0x840 = 1); not ported. The gu_set root path isn't applied, and the crossfade (if any) on button presses isn't verified → B40b3.
- The zero-colour third light is ignored (no visible effect).
- Brightness/colour: matched by eye on the side-by-sides only, not pixel-compared.
- Costumes other than 0: not compared against the original's Costumes tab; umpire previews not done → B40b4.
- Mods: donor pose on a rerigged skeleton checked by eye for 3 Fore! mods only; their face channels doki_eye/doki_mouth are missing (mod side warning).

# B40b1 — previews in the select (2026-10-08)

## What was built
- `inspect.rs`: `spawn_into` draws a preview into its own float image (`target`) instead of the window; a UI node
  shows it through `PreviewMaterial` (`inspect.wgsl`), which decodes the GS-encoded values for the sRGB HUD, so the
  card's text (style, name, Ready) stays on top as over the original's portrait. The image clears to alpha −1:
  coverage = alpha + 1 (the model's own alpha is its texels' / VU1 specular, not coverage). `act` (any pad) now
  belongs to the `--inspect` viewer only; `plugin` is placement + material.
- `play/main_menu/previews.rs` (new): each card a player is picking on emits `Tex::Preview(p)` in place of the
  portrait; `sync` spawns/respawns that card's preview on hover or costume change (loaded characters cached by
  (character, costume)), places it on the card's 128×160 rect and stacks it by quad order (every slot's `ZIndex` =
  its layout index). `buttons` routes each seated player's own R2 / L2 / △ (keyboard: 1, 2, the attributes key) to
  `inspect::apply`. `HST_SELECT=c,c,c,c` (with `HST_MENU=chars`) seats all four players hovering those characters.
- `main_menu.rs`: only the `Tex::Preview` variant, the card's portrait quad swapped for it, and `draw` skipping it.

## Verification
`tools/shot.sh` (`--shot` works on the menu window now) with `HST_MENU=chars HST_SELECT=…`, four shots covering all
14 characters: `context/b40b1/sel_{a,b,c,d}.png`; each card cropped beside the original's inspect screen
(`orig_cNN.png`, B40b's PCSX2 captures): `context/b40b1/side0.png`, `side1.png`. Every pose, facing, racket hand
and light matches by eye. `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- R2 / L2 / △ in the select: wired per seat but not pressed in a live run (shots are non-interactive); `apply`
  itself is B40b's, checked in `--inspect`.
- Mods: the select doesn't list B40a's mods yet (B40a blocked); `spawn_into` takes a mod's `CharacterData` and donor
  as `--inspect` does, so B40a only has to feed its picks to `sync`.
- The card shows the inspect screen's model column (`inspect::CROP`, head to feet) widened to the card's aspect,
  not the original select's hand-made portrait crop (a deliberate change: the user asked for models here).
- Translucent model draws over the −1 clear come out as 2a − 1 (alpha) over black: their edges can darken a little;
  no MSAA on select previews (the GS doesn't antialias; the `--inspect` viewer keeps Bevy's MSAA).
- The preview image is sized for the window when spawned; a window resize keeps the old resolution until the next
  hover/costume change.
- Costumes other than 0 aren't compared against the original (B40b4).

# B40b3 — inspect gu/di details

## Findings in the original (menu overlay + PCSX2 copy 4, Data → Items → Characters, scratch slot 9)

- The inspect object's draw (0x3230e0) takes one of two paths: +0x840 = 0 → the plain draw (0x37d160: the model's matrix from its spot, yaw and hand mirror); +0x840 = 1 → 0x37d280: the same matrix, the skeleton posed with it, `Bip01Pelvis` (string at 0x3888f8) looked up, and the matrix moved by (spot − pelvis world) in x and z (y 0) before drawing. So the pelvis sits over the spot (0, 8).
- +0x840 is set to 1 by the R2 and L2 handlers (0x3476e0) when the character is 5 or 13, else 0. △ (0x1000) doesn't touch it, so after a reaction the pose is pelvis-centred too. A character or costume change (0x34ce10) and the screen's init (0x322f70) clear it.
- No root path: the menu loads only `re_pcNN_gu_set/di_set.ANI2` (+ .MOR/.UVA) from `MENU/PC/PCnn.XB` (byte-identical to PCANI's), no `_dummy` path. Live for char 0: the root matrix stays at (0, 1.5, 8) through gu_set (context/b40b3/c0_r2.tsv). The B40b gap note "apply gu_set's root path" was wrong; nothing to apply.
- No crossfade: both switches call the motion setter 0x140360 with a null blend source, and its blend copy (0x140a70) returns at once without one. It's a cut, as the port already does.
- Live for char 5 (context/b40b3/c5_r2.tsv, `research/b40b3_probe.py`): with +0x840 = 1 the pelvis world x/z stays (0, 8.0) and the root z moves 7.79–8.03 over gu_set; after △ the root is (−0.000284, 8.001564). The motion clock reads one frame ahead of the drawn joints (it ticks after the draw).

## Built

- `inspect.rs`: `Preview::centre` (set by `apply` for Win/Loss on chars 5/13, kept through Pose; a respawned preview starts false), `pelvis_centre(skeleton, clip, t, yaw, hand)` (the pelvis's world point through `hst_sim::pose::node_world`, model x/z = 2·spot − pelvis) and a `centre` system after `character::animate` that moves the rig for the drawn time. `apply` now takes `&mut Preview`.
- Test `inspect::tests::pelvis_centre_matches_the_game`: Brad's pose (66) and gu_set frames 0, 12, 15, 59, 120 against the recorded root x/z, within 1e-5.

## Not verified / not 1:1

- Float exactness: `pelvis_centre` uses Rust `sin_cos` for the yaw and plain f32 for the shift, not the game's sin and `hst_sim::ps2` ops. It matches to ~1e-6 on the recorded frames, not bit-exact (drawing only).
- Char 13 and L2 (di_set) weren't recorded live. Same handler branch and draw path per the decompile.
- Frame phase: the original draws clock − 1. The port draws its own interpolated clock (general motion timing, not this task's).
- No side-by-side screenshot: `--shot` can't press R2, so the centring is checked by the test only.

# B40b2 — eye UV animation

What was built:

- `character.rs`: `disc_motions` loads each motion's `.UVA` next to its `.MOR`, the pose's too (`pcNN_pose.UVA`), into `Face::uv`.
  - Tracks are bound to a skeleton node by name.
  - `uv_length` is the last key over all UVA tracks.
- `CharacterData::part_nodes` lists, per part, the nodes its packets' palettes hold, plus the part's `uv_swap`. A UVA track named after a node drives every packet holding that node, as in `court_anim`.
- `inspect.rs`: the UVA plays on its own clock, `UvClock`.
  - Pose: held at the pose frame, wrapped into the UVA length (looping, speed 0).
  - R2/L2: from 0 at speed 1, clamped at the end.
  - A motion without a UVA draws no offset.
  - `uv_offsets` writes the sampled (x, y), or (y, x) for swapped parts, into each part's `GsMaterial.uv_offset`, second draws included.
- `main.rs`: `--inspect` no longer also opens the main menu (the `menu` condition ignored `roster`).

Findings in the original (menu overlay decompilation):

- The anim object has six controller slots; slot 1 plays the UVA.
- The pose start selects UVA 0 with loop on and sets its time to the pose frame. The set-time call wraps or clamps like `hst_sim::pose::wrap`. Speed stays 0, so the time is held.
- gu/di select their UVA with loop off, time 0, speed 1.
- Switching to an index with no file unhooks the old bindings, so the offset goes back to 0.
- The menu `taguchi` gu_set/di_set files are byte-identical to PCANI's. UVAs exist for gu_set of characters 1, 2, 3, 4, 8, 10 and 12, and di_set of 3 and 8.
- Bindings, costume 0: every pose UVA has a single eye track.
  - It names one node (`s_me`, `eye`, `oki_eye`, `mai_me`, `eyes`, `eye1`, `c_me`), covering whole materials.
  - No eye packet is stored swapped.
  - pc07's track names a node its model lacks, so it binds nothing (in the original too: no match, no hook).
  - pc13 has no pose UVA.

Verified on PCSX2 copy 3, slot 9 (Items screen, Ashley's pose preview):

- EE RAM holds Ashley's eye packet's VIF unpack (V4-32, double-buffered at two addresses) carrying (−0.02, 0.02, 0, 0), bits 0xbca3d70a / 0x3ca3d70a.
- That is exactly what the port samples at the held frame 69, in the same (x, y) order.
- The test `inspect_poses_bind_and_hold_the_games_frames` asserts those bits and that every pose but pc07's and pc13's binds a UV track.

## Not verified / not 1:1 (B40b2)

- Only character 0's held value was read live. The others are computed the same way from their files, but not compared against RAM.
- R2/L2 UVA playback (characters 1, 2, 3, 4, 8, 10, 12) was not recorded live; its timing follows the decompilation only.
- The sampler cursor restarts at key 0 each sample. The game keeps a cursor, which only matters when a sample lands exactly on a key; the port takes the segment before it.
- Each part gets one swap flag, from its first packet. All eye packets are unswapped, so this has no effect today.
- Mods get no UV binding (`part_nodes` is empty for mods): a rerigged mod has no packet palettes → M1i.
- In-match per-motion UVAs are loaded into `Face::uv` but only the inspect previews play them; the in-match face path is unchanged.
- `tools/check.sh`: `mods::tests::rerigged_mod_plays_forehand_and_run` fails on the sway assertion (`noise`: mod None vs disc Some(3, 126)). This diff doesn't touch sway. The local `context/mods/test_pc00` fixture is likely an export from before M1d's sway; not re-exported.

# B40b4 — costume and umpire previews (2026-10-08)

## Costumes

All 14 characters × 10 costumes compared: the original's Data → Items → Costumes tab (PCSX2 copy 3, scratch slot 8,
driven by a script that sets the tab's character and costume bytes and shoots each) against the port's
`--inspect c:k` (one costume per run). Model, textures, colours, pose and racket hand match for all 140. The previews
were already in place (B40b1 loads any costume through `load_disc`); no code change. Shots: `context/b40b4/orig/`,
`context/b40b4/port/`, side-by-side crops `context/b40b4/side_cNN.png`.

## Umpires: the original has no 3D umpire previews

- Data → Items → Umpires is not a 3D object. It draws the pre-rendered sprite `MENU60.XB0 item_umpireNN.tm2`
  (256×512, the umpire on the chair). The tab screen's inspect object is used only for the Characters and Costumes
  tabs; on the Umpires and the next tab it is hidden. So it is not the same object as the inspect screen, and there is
  no pose, camera or light to port.
- The real umpire select (confirm screen → Select Umpire, `context/b40b4/ump1.png`) is 2D too: a row of five face
  cards (Chika, Suzuki, Lily, Robot Tennis, Anna), the hovered card drawn larger and raised with the hand on its left,
  a "△ Voice Sample" pill at the right, the info line "Choose an umpire for this match.", and the face plus name at
  the top right. No 3D models anywhere on that screen.
- So no 3D umpire preview was built: its pose, camera and light would all be guesses. The 1:1 face-card select is
  B64's / B40e's job; B64's "umpires posed and animated" wording was corrected to match.

## Not verified / not 1:1

- The original's shots are under PCSX2's widescreen patch, which squeezes its 3D ×0.75 across; framing was compared
  with that in mind, not pixel for pixel.
- Costume lighting compared by eye only (no light values read for the Costumes tab beyond B40b's).
- No 3D umpire previews: the original has none (see above). If the remaster wants them anyway as a deliberate
  change, that needs the user's call and its own task.

# M1i — eyes for modded characters (2026-10-12)

## Built
- `character.rs`: `Face::uv_unbound` keeps the `.UVA` tracks whose node the skeleton lacks (by name; a disc
  character still never plays them). `EYES` (a sentinel node) and `disc_nodes(iso, n)` (the body model's node names).
- `mods.rs`: mods now fill `part_nodes`: per primitive, the joints its vertices are weighted to (the glTF analogue
  of a disc packet's palette), so a costume that keeps the donor's eye node as a joint (a rerigged HST model:
  test_pc00 keeps `s_me`) binds the donor's eye track by name exactly like the disc. New optional `mod.json`
  `"eyes": ["<material>", …]` (checked in `read`; a name missing from a costume warns): those primitives also hold
  `EYES`, and the donor's unbound eye tracks that name a node of the donor's own model are rebound to `EYES`. A
  track naming no node of the donor's model (pc07) stays dropped, as on the disc.
- `inspect.rs`: the offset maths moved out of the `uv_offsets` system into `part_uv(data, face, t)` (unchanged).
- HST-MODS `standard/HST-CHARACTER-STANDARD.md` §4c *Eyes* + the `eyes` key in §6 (HST-MODS 11546cd).
- Test `mods::tests::mod_eyes_follow_the_donors_uva`: on the inspect pose, test_pc00's moved parts carry exactly
  the disc pc00's offsets (bits 0xbca3d70a/0x3ca3d70a, as the original's VIF packet); Fore!'s Phoebe (no donor eye
  node) moves nothing without `eyes`, moves only its `eye` material's parts with donor 6's offset with
  `"eyes": ["eye"]`, and nothing with donor 7.

## Not verified / not 1:1
- In-match motions: the faces carry every motion's UVA for mods as for disc characters, but nothing draws UVA in
  a match for any character yet (only the inspect previews) → new task B40b5.
- No screenshot of a mod's eyes: the offset is proven equal to the disc's by the test; whether a foreign mod's eye
  texture looks right under the donor's offsets depends on its texture layout (the standard tells authors to lay
  it out for them). No packaged mod sets `eyes` yet (the HST-MODS packagers weren't changed).
- Palette `part_nodes` for mods use weight > 0 joints of each primitive, not the PS2 packet palette (mods have none).
- `tools/check.sh` at wrap-up: 138 pass, 1 unrelated failure: `play::mod_match::tests::get_a_grip_stats_reach_the_match`
  asserts Emi's `ai_row` ≠ `donor`, but `context/mods/getagrip_pc00_emi/mod.json` was repackaged today (10:20,
  M1l sizing) with donor 0 = ai_row 0. Local data vs a stale assertion, not touched here (M2i's test).

# B40b5 — eye UV animation in the match (2026-10-12)

## Found (match decompile, the motion setter)
- The setter that starts a player's motion picks the face index once (motion number; +0x1404 for team
  reactions ≥ 0x30 unless the co flag matches; 0x1c/0x1d keep the old one) and selects it in **both** slot 0 (MOR)
  and slot 1 (UVA). It then copies the same loop flag and speed into every controller and sets every controller's time
  to 0. So the UV player is the morph player's clock (same restart, loop and speed) wrapped/clamped by the `.UVA`'s
  own length. An index without a `.UVA` unbinds, so the offset is 0 (as in the menu).
- Correction to the N1d4/B40b2 notes ("only gu_set uses UVA"): bound in-match UVAs exist for reactions 0x2c..0x2f
  of characters 1, 2, 4, 6, 8, 10, 12 (various), and for 24 motions of character 3 (stance 0..2, the strokes
  0x10..0x17, whiffs, reactions). Test `character::uv_tests::match_reactions_play_the_eyes_uva` pins the list.

## Built
- `character.rs`: `Motion::uv_clock`. `set` restarts it as the face clock, `play` copies speed and loop into it,
  `tick` keeps its speed equal to the face clock's (the serve follow-through sets only the face clock's) and ticks it
  by `uv_length`. New system `character::uv_offsets` (registered in `main.rs` next to `texture_faces`) writes
  `inspect::part_uv` into each part's `GsMaterial.uv_offset` for every rig except the inspect previews, which
  keep their own `UvClock`. Rigs without `part_nodes` (umpire, NPCs) are skipped. Mods use the same path (M1i's
  `EYES`). No sim state is read or written; `play.rs` is untouched.

## Not verified / not 1:1
- No live PCSX2 comparison of an in-match eye offset. The timing comes from the decompiled setter only.
  In-match presentation needs no PINE proof (PLAN), and the sampler/binding is the one B40b2 verified live.
- No side-by-side screenshot: a single `--shot` frame can't show scrolling, and the offsets are small (±0.02 UV).
- "At contact all controllers are set to time 5.0" (N1d4) isn't ported for the face or UV clock. It was a gap
  before this task too (the face clock doesn't do it either).
- The UV offset is sampled at the tick's sampled time, not interpolated within a frame (same as the face weights).
