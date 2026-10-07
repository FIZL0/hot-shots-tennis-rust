# P7 — Exact character movement (P7a: human pad → run, Carol/Kaito replay)

Code: pad read 188880 (per-player pad struct: raw 18-byte pad at +0x24, lx +0x28, ly +0x29), human direction 348ff0 (called from 3467b0 when player +0x84 == 0; bots get a byte-quantised AI direction instead), play-state update 349410 (state +0x3fa4: 0 → 349410, 1 → 351dd0, 2 → 354050). Asm in the session scratchpad (DumpAsm of 188880/348ff0/349410).

## Pad → run direction (ported: `hst_sim::player::pad_dir`)
- axis(b) = b − (b ≥ 0x81) − 0x7f → −127..127. Quirk: centre 0x80 reads +1 (seen: lx=ff, ly=80 gives vel z −0.00069).
- +0x50/+0x54 = (i − sign·48)/79 when |i| ≥ 48 else 0 — only used as "stick inside the dead square" test.
- +0x60: fx = ix/127, fz = −iy/127; n = 1/sqrt(fz² + fx² (mula/madd)); mag = max(|fx|,|fz|); m = 0 if mag ≤ 0.37795275 (0x3ec18306) else (mag − t)/0.62204725 (0x3f1f3e7d); vector (fx·n·m, 0, fz·n·m).
- 348ff0: both stick axes inside |48| → d-pad (byte +0x41, 2-bit fields up/right/down/left; right beats left, up beats down): x ±1, z ±1, normalised if length > 1. Then ×1.2 (0x3f99999a), normalised if sqrt(z² + x²) > 1. Then × camera matrix: identity, or rotated π about y when the camera is on the +z side (flag set from camera z ≥ 0; slot 4 camera z = −39 → identity). The identity multiply turns −0 into +0 (needed for bit-exact velocity z). Zeroed unless phase gm+0x55 ∈ {2,3,4}.
- The pad of the same frame applies (no one-frame lag).
- Hand: +0x12b4 comes from the model object flag, not TParam: in slots 3/4 Carol (TParam 左) has +0x12b4 = +1.0, Will −1. play.rs `character_hand` reads TParam col 6 — open (P7b).

## Verification (tests/player.rs `human_pad_replay`)
Fixtures (context/fixtures, not in git): p7_carol_s04.bin (420 samples, slot 4, Carol P1 with a scripted stick/d-pad script: full/partial stick, dead zone, diagonals, d-pad diagonals and opposites), p7_kaito_s04.bin (300 samples, scratch slot 8 = slot 4 with player 0 given Kaito's speed 1.1 (+0x1374) and agility 25 (+0x1388); Carol's body/pelvis kept — movement only depends on speed, agility, stamina, hand), p7_pelvis_s04.bin (player 0's 48 pelvis rows, +0x6b0 + m·0x40). Stats from TParam rows 6 and 3 (agility asserted equal to the recording). Body::step run continuously over every play-state frame (resync only when play state starts): Carol 232 frames (119 running), Kaito 219 (90 running) — position, velocity, motion, facing all bit-exact. PCSX2 copy needs NominalScalar 0.5 to record with no missed frames.

## play.rs
`pad_run` (new): app stick → pad bytes → `pad_dir`, flipped when the camera eye is on +z; used by `human()` for running. Ponytail: the app merges d-pad/keys into the stick, so the d-pad's faster diagonal path isn't used; world axes under the free camera.

## Per-character stats (TParam.csv; speed = SPE/10)
| row | name | hand col 6 | SPE | STA | costs dive/back/smash | Agili | col23 Body ADJ | col25 Back ADJ | col26 Rizing ADJ | col27 Run CON |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Eleanor エレノア | R | 10 | 40 | 8/4/7 | 40 | 100 | 100 | 74 | 0 |
| 1 | Yuuki ユウキ | R | 9 | 40 | 6/3/7 | 40 | 100 | 100 | 90 | 0 |
| 2 | Jun ジュン | R | 11 | 40 | 8/4/7 | 40 | 100 | 100 | 78 | 0 |
| 3 | Kaito カイト | R | 11 | 40 | 8/4/7 | 25 | 100 | 100 | 74 | 0 |
| 4 | Momoko モモコ | R | 10 | 40 | 8/4/7 | 30 | 100 | 100 | 74 | 0 |
| 5 | JJ | R | 12 | 40 | 8/6/6 | 10 | 100 | 100 | 90 | 0 |
| 6 | Carol キャロル | L | 12 | 40 | 6/4/7 | 35 | 100 | 100 | 90 | 0 |
| 7 | Bull ブル | R | 9 | 40 | 8/3/8 | 20 | 100 | 100 | 78 | 0 |
| 8 | Miranda ミランダ | R | 12 | 40 | 8/4/7 | 25 | 100 | 100 | 85 | 0 |
| 9 | Ryu リュウ | R | 10 | 40 | 6/3/10 | 30 | 100 | 100 | 90 | 0 |
| 10 | Laura ローラ | R | 11 | 40 | 8/4/7 | 25 | 100 | 100 | 78 | 0 |
| 11 | Will ウィル | L | 11 | 40 | 8/4/7 | 25 | 100 | 100 | 74 | 0 |
| 12 | Gloria グロリア | R | 10 | 40 | 8/4/7 | 35 | 100 | 100 | 74 | 0 |
| 13 | Suzuki スズキ | R | 11 | 35 | 8/6/8 | 25 | 125 | 100 | 90 | 0 |

(Column-to-name mapping of 23/25/26/27 is from the CSV header; how they turn into numbers is P7c.)

## P7b
- Hand source: the TParam.csv parser sets right only when col 6 is exactly 右 (else left). The model's left-hand flag (+0x135) = left XOR the player's character-select "switch hand" toggle (byte +6 of the per-player selection record). +0x12b4 = +1 when the flag is 0, else −1. Slots 3/4 P1 has the toggle on, so Carol (左) plays +1. Slot 2's menu P1 has it on too: every pick from there plays opposite to TParam (Carol and Will right, the rest left). play.rs `character_hand` now compares with 右 like the game. The toggle is always off there because there's no character select yet (ponytail comment).
- Choosing and watching characters (for every agent): `tools/pick.py <char> [slot]` starts a doubles match from slot 2 with that character as P1 and saves the serve. Its menu paths are checked for all 14 characters. `tools/screenshot.sh` uses the copy's own F8 when HST_PCSX2 is set. Both are documented in plan/REFERENCE.md → Controlling the real game, along with what isn't automated yet (COM characters, costume, switch-hand toggle, singles, P1 receiving).
- Recording: `research/p7b_record.py <char> <dir> [frames=1500]` (under tools/pcsx2.sh). It runs pick, dumps hand/flag/selection and the 48 pelvis rows, then records 6 cycles of serve (○ ×3: the first press after a load or point is ignored, then toss, then hit) plus scripted stick/d-pad running. Fixtures (context/fixtures, not in git): p7b_cNN.bin + p7b_cNN_pelvis.bin for NN = 00–13. 0–9 and 12–13 are 1000 samples (4 cycles); 10 and 11 are 1500 (6 cycles, after 4 cycles gave under 50 running frames).
- Results: `human_pad_replay` steps all 14 characters with their own bodies and hands, 223–378 frames each (122–229 running), all position/velocity/motion/facing bit-exact. Frames with a face button held are skipped: a press in play starts a swing (P5), not running. `pelvis_table_from_disc` now also checks the 14 dumped pelvis tables against the disc (worst 4.8e-7).
