# HST character mod standard (v1, draft)

What a custom character for this Hot Shots Tennis remaster must provide, so one loader can treat
it exactly like a disc character. It mirrors `crates/hst/src/character.rs` (`CharacterData`): a custom character
"only has to fill the same CharacterData". Machine-readable skeleton: `modding/hst_skeleton.json`
(regenerate: `python3 modding/tools/make_skeleton.py DIR`, DIR from `hst-gltf hst`). How-to: `modding/TUTORIAL.md`.

## 1. Mod folder

```
mods/<id>/
  mod.json                 manifest (§6)
  model/<costume>.glb      one per costume, §2–§4 (c00 required)
  racket.glb               optional rigid racket in Racket-joint space (else the donor's)
  textures/upscaled/       optional: same file names as the PNGs embedded in the .glb, used when the
                           global "upscaled textures" switch is on (B40); missing files fall back to native
  voice/<program>_<key>.wav  §5
  face.json                only for texture faces (§4b)
```

## 2. Model file (.glb, glTF 2.0)

- Scene root node `game_space`, rotation `[0,0,1,0]` (180° about Z). Everything under it is in **HST game
  space**: metres, **Y down**, feet at y = 0, character facing +Z (same as every exported HST character).
- One skin. Skinned vertices are in game space; inverse bind matrices are game space too. **Rest pose = bind
  pose** (node locals compose to the inverse of the IBMs).
- ≤ 4 influences per vertex (JOINTS_0/WEIGHTS_0), weights sum to 1.
- Materials: base-colour PNG embedded; `KHR_materials_unlit` allowed (the GS path lights per vertex anyway).
  Texture alpha is *not* coverage on HST bodies (it holds shading masks); mark real cut-outs `alphaMode: MASK`.

## 3. Skeleton

The **54 core joints** of `hst_skeleton.json`, with exactly these names and parents (HST's 3ds Max Biped, spaces
removed): `Bip01` (root) › `Bip01Pelvis` › `Bip01Spine` › `Spine1` › `Spine2` › `Neck` › `Head`; clavicles under
**Neck**; thighs under **Spine**; `Hand` › `Finger0..4` with three segments each (`Finger0`, `Finger01`,
`Finger02`…); `Foot` › `Toe0`; `Racket` under `Bip01RHand`.

- **Bone axes must match HST's**: each joint's bind frame is HST pc00's frame swung onto the model's own bone
  direction (joint → its child). HST motions store *absolute local rotations* (`hst_sim::pose::Clip`), so a frame
  that differs (e.g. a rotation-reset rig) poses the mesh wrongly even with correct names.
- **Proportions are free.** The game scales motion positions to the model: `Bip01` by its rest height |y|,
  others by rest bone length, `Bip01Pelvis` not at all. HST's own range: `Bip01` height 0.58–0.95 m (see
  `characters` in the JSON). Keep `TParam` reach values (§6) consistent with the arm length.
- **Extra joints are allowed** (hair, skirts, earrings, accessories) anywhere below the core. HST motions don't
  drive them; they follow their parent (sway via `.NOI`-style deformers is a later extension).
- `Racket`: where the racket's grip sits; the racket mesh is rigid in that joint's space.

## 4. Face

### 4a. Morph faces (preferred — what HST uses)

Morph targets on the head/face mesh, names in `mesh.extras.targetNames`, **without** the PS2 object prefix
(HST stores `face\x01joy_eye`; mods store `joy_eye`; the loader matches the part after `\x01`):

| channel | required | meaning |
|---|---|---|
| `joy_eye`, `joy_mouth` | yes | happy (point won, win) |
| `anger_eye`, `anger_mouth` | yes | angry / frustrated |
| `sorrow_eye`, `sorrow_mouth` | yes | sad (point lost, loss) |
| `doki_eye`, `doki_mouth` | yes | flustered / surprised |
| `blink_eye` | recommended | blink (pc07 and pc13 have none) |

Weights 0–1, full expression at 1. HST plays the faces from the **donor's** `.MOR` tracks (§6) bound by these
names; extra targets (`smile_*`, `wink_eye`…) are kept but only play if the donor's tracks name them. A missing
required channel freezes that part of the face; fill it with the closest expression (rerig does:
joy←smile/laugh, anger←vexing, sorrow←tukare, doki←surprise/open).

### 4b. Texture faces (PSP sources: Get a Grip, Open Tee)

These games swap one whole-face texture (eyes + mouth together) instead of morphing; Get a Grip's face mesh has
two materials, `face` and `faceL` (mirrored halves; they differ only for winks). `face.json`: `{"materials": ["face","faceL"], "neutral": "<png>", "channels":
{"joy_eye": "<png>", "joy_mouth": "<same png>", …}}` — the loader shows the texture of the strongest channel
above 0.5, else neutral. (Needs loader support: PLAN M1b.)

## 5. Voice

HST plays per-character voice **programs** (each a set of keys = alternative takes) from
`SND/VOICE/PC/PCnnVCEk.XB0` (singles a/b, doubles a/b, 70/30 random). Ship `voice/<program>_<key>.wav`:

| program | when | keys |
|---|---|---|
| 0 | strong-toss serve / smash | 0–2 |
| 1 | good stroke shout | 0–4 |
| 2 | mis-hit | 0–2 |
| 3 | dive | 0–2 |
| 4 | whiff | 0–1 |
| 6 | partner / first-serve calls | 0–4 |
| 7–10 | post-point reactions (by reaction motion 0x2c–0x2f) | any |

(From `research/journal/2026-10-07-n3-audio/9-SHOUTS-FINAL.md`, `10-WHIFF-CALLS-FINAL.md`.)
Get a Grip's labelled cues map almost 1:1: smash→0, swing→1, receive→3, point_get/point_lost→7–10, start→6.

## 6. mod.json

```json
{
  "standard": 1, "id": "gag_emi", "name": "Emi", "source": "Hot Shots Tennis: Get a Grip pc00",
  "costumes": ["model/c00.glb"],
  "donor": 0,
  "hand": "right",
  "params": {"base": 0, "override": {"Serv POW": 4, "SPE": 10}},
  "ai_row": 0,
  "face": "morph",
  "voice": "voice/"
}
```

- `donor`: HST character (0–13) whose **motions, `.MOR` faces, arm table, shot records and AI rows** are used.
  Pick by body type and play style (`research/characters.md`).
- `params`: `TParam.csv` row: `base` donor row, `override` by column header (e.g. `Serv POW`, `Strk POW`,
  `Voley POW`, `Top SPIN`, `Strk CON`, `SPE`, `STA`, `Agili`, reach `リーチ(cm)`, heights). Other games' stats are
  mapped once per game (PLAN M8).
- `hand`: `right`/`left` (the game mirrors left-handers, like Carol and Will).
- `face`: `morph` (§4a channels), `texture` (§4b, needs `face.json`) or `none` (no face animation).
- The HST loader (`crates/hst/src/mods.rs`) rejects a mod with `<path>: <reason>` when the manifest is off
  (`standard` ≠ 1, a missing costume, `donor`/`base`/`ai_row` outside 0–13, `hand` not right/left, an `override`
  key that is no TParam column) or a costume fails §7's FAIL checks; WARN checks are logged. Buffers and images
  must be embedded in the `.glb`; it skips glTF accessor validation (older rerig output lacks POSITION min/max).

## 7. Conformance

`python3 modding/tools/rerig.py SRC OUT --preview DONOR.glb` writes a conforming model, `modding/tools/check.py OUT`
checks it, and `modding/tools/render.py OUT out.png --anim sh_pc00_f_t --times 0,0.4` renders it.
A file conforms when: root `game_space`; all 54 core names/parents; rest = bind; ≤ 4 weights; required face
channels (or a `face.json`); an HST forehand and run render without tearing or twisted limbs.
