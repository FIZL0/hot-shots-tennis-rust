# M1r — every mod holds its own racket again

## What changed
- `mods.rs`: `mod.json` `disc_racket` (0–13, optional, checked in `read`): with no `racket.glb` the mod holds that
  disc character's racket (`PC/PCnnC00.XB` `racket_nn`, read from the user's disc) instead of its donor's. Order:
  racket.glb → disc_racket → donor. Looks only (the racket mesh never reaches gameplay; handshake hash untouched).
- ../HST-MODS (commit 2862b75) `tools/rerig/package.py pick_racket`, standard §1/§6, README, build.sh,
  notes/getagrip.md. All 87 mods repackaged.

## Sources
- No source game has a per-character racket: Get a Grip's Racket_param has no character column (notes/getagrip.md);
  the golf games have no rackets. So every racket is a pick, except Gloria and Suzuki, who are HST characters too
  and get HST's own Gloria's (12) / Suzuki's (13).
- Pool: 24 distinct rackets. Get a Grip's 10 models (`racket00`–`08` = items Racket_0N0–0N2, the three colours of a
  model share it; `racket50` = Racket_500), md5-distinct textures; HST's 14 disc rackets (`racket_00`–`13`, every
  MDL and tex0 distinct). Fore has 24 mods, so it uses all of them.

## Rule (pc order, first free in the game)
1. Gloria / Suzuki → HST's same character (reserved in games that have them).
2. Get a Grip mods: Get a Grip's models in order (Emi keeps Racket_000 "Standard"), then the donor's HST racket,
   then HST's in order. Packaged with the source grip (racket.py as before).
3. Golf mods: the donor's HST racket (the character whose motions it plays), then Get a Grip's models (Emi's grip
   carried hand to hand, as before), then HST's in order.
Each mod's `racket_why` says which and why.

## Mapping
## fore: 24 mods, 24 rackets, dupes []
  fore_pc00_phoebe             HST disc 6       donor 6
  fore_pc01_mike               HST disc 1       donor 1
  fore_pc02_emma               GaG racket00     donor 6
  fore_pc03_sam                GaG racket01     donor 1
  fore_pc04_misaki             GaG racket02     donor 6
  fore_pc05_brad               GaG racket03     donor 1
  fore_pc06_chaos              HST disc 11      donor 11
  fore_pc07_regis              GaG racket04     donor 1
  fore_pc08_maya               GaG racket05     donor 1
  fore_pc09_falcon             GaG racket06     donor 1
  fore_pc10_renee              GaG racket07     donor 6
  fore_pc11_z                  GaG racket08     donor 11
  fore_pc12_mel                GaG racket50     donor 6
  fore_pc13_allan              HST disc 0       donor 1
  fore_pc14_tiffany            HST disc 2       donor 6
  fore_pc15_kamala             HST disc 3       donor 6
  fore_pc16_toni               HST disc 4       donor 1
  fore_pc17_tbone              HST disc 5       donor 1
  fore_pc18_lin                HST disc 7       donor 6
  fore_pc19_louise             HST disc 8       donor 6
  fore_pc20_zeus               HST disc 9       donor 1
  fore_pc21_hubert             HST disc 10      donor 1
  fore_pc22_ratchet            HST disc 12      donor 1
  fore_pc23_jak                HST disc 13      donor 1
## getagrip: 15 mods, 15 rackets, dupes []
  getagrip_pc00_emi            GaG racket00     donor 0
  getagrip_pc01_ban            GaG racket01     donor 1
  getagrip_pc02_taiga          GaG racket02     donor 1
  getagrip_pc03_wendy          GaG racket03     donor 12
  getagrip_pc04_brad           GaG racket04     donor 11
  getagrip_pc05_rosetta        GaG racket05     donor 6
  getagrip_pc06_mitsuzane      GaG racket06     donor 1
  getagrip_pc07_paola          GaG racket07     donor 6
  getagrip_pc08_fan            GaG racket08     donor 1
  getagrip_pc09_rachel         GaG racket50     donor 12
  getagrip_pc10_schneider      HST disc 11      donor 11
  getagrip_pc11_norma          HST disc 0       donor 12
  getagrip_pc12_suzuki         HST disc 13      donor 1
  getagrip_pc13_gloria         HST disc 12      donor 12
  getagrip_pc14_helghast       HST disc 1       donor 11
## oob: 17 mods, 17 rackets, dupes []
  oob_pc00_jasmine             HST disc 0       donor 0
  oob_pc01_nick                HST disc 1       donor 1
  oob_pc02_sophie              HST disc 6       donor 6
  oob_pc03_bjorn               GaG racket00     donor 1
  oob_pc04_sasha               GaG racket01     donor 0
  oob_pc05_fernando            HST disc 11      donor 11
  oob_pc06_sonia               GaG racket02     donor 12
  oob_pc07_felipe              GaG racket03     donor 11
  oob_pc08_bloom               GaG racket04     donor 11
  oob_pc09_lj                  GaG racket05     donor 11
  oob_pc10_kate                GaG racket06     donor 12
  oob_pc11_dino                GaG racket07     donor 11
  oob_pc12_anya                HST disc 8       donor 8
  oob_pc13_suzuki              HST disc 13      donor 1
  oob_pc14_s_maruyama          GaG racket08     donor 1
  oob_pc15_alex                GaG racket50     donor 0
  oob_pc16_gloria              HST disc 12      donor 12
## opentee2: 21 mods, 21 rackets, dupes []
  opentee2_pc00_mika           HST disc 0       donor 0
  opentee2_pc01_rio            HST disc 1       donor 1
  opentee2_pc02_alia           HST disc 6       donor 6
  opentee2_pc03_shu            GaG racket00     donor 1
  opentee2_pc04_julie          GaG racket01     donor 0
  opentee2_pc05_cj             GaG racket02     donor 1
  opentee2_pc06_patricia       GaG racket03     donor 12
  opentee2_pc07_logan          HST disc 11      donor 11
  opentee2_pc08_holly          GaG racket04     donor 12
  opentee2_pc09_roger          GaG racket05     donor 11
  opentee2_pc10_carly          GaG racket06     donor 0
  opentee2_pc11_luke           GaG racket07     donor 1
  opentee2_pc12_alice          GaG racket08     donor 0
  opentee2_pc13_malachy        GaG racket50     donor 1
  opentee2_pc14_frauada        HST disc 2       donor 6
  opentee2_pc15_montcalm       HST disc 3       donor 1
  opentee2_pc16_lauryn         HST disc 4       donor 12
  opentee2_pc17_mifune         HST disc 5       donor 11
  opentee2_pc18_lee            HST disc 7       donor 12
  opentee2_pc19_chad           HST disc 8       donor 11
  opentee2_pc20_gloria         HST disc 12      donor 12
## opentee: 10 mods, 10 rackets, dupes []
  opentee_pc00_mika            HST disc 0       donor 0
  opentee_pc01_rio             HST disc 1       donor 1
  opentee_pc02_alia            HST disc 6       donor 6
  opentee_pc03_shu             GaG racket00     donor 1
  opentee_pc04_julie           HST disc 12      donor 12
  opentee_pc05_cj              GaG racket01     donor 1
  opentee_pc06_patricia        GaG racket02     donor 0
  opentee_pc07_logan           HST disc 11      donor 11
  opentee_pc08_holly           GaG racket03     donor 12
  opentee_pc09_roger           GaG racket04     donor 11

(`GaG racketNN` = racket.glb from Get a Grip's part; `HST disc N` = `disc_racket` N: 0 Ashley, 1 Cody, 2 Jun,
3 Kaito, 4 Momoko, 5 JJ, 6 Carol, 7 Big Chief, 8 Miranda, 9 Kent, 10 Lola, 11 Will, 12 Gloria, 13 Suzuki.)
No two mods of a game share a racket (checked over every mod.json/racket.glb).

## Checks
- `mods::tests::checks_the_manifest` (disc_racket 14 rejected, 12 read), `disc_racket_is_the_named_characters`
  (GaG Suzuki, donor Cody, draws HST Suzuki's racket's vertices, not Cody's), `loads_every_mod` (all 87, --ignored).
- `--play --singles --mod` shots with a close `HST_EYE` (context/m1r/): Ban with GaG racket01 (red), Suzuki with
  HST Suzuki's, Phoebe (left-handed) with Carol's in the gloved left hand, Fore Emma with GaG racket00 on Emi's
  carried grip. All in hand.

## Not verified / not 1:1
- The picks themselves are ours: no source data names a racket per character (above), so nothing to match.
- A disc racket on a mod sits in the mod's `Racket` joint as it does on its own character (all HST rackets are
  0.944 m with the same grip); not compared against a source game, which has none for these characters.
- Get a Grip's per-model colour variants (Racket_0N1/0N2) aren't in the exported parts (no separate textures found),
  so only 10 Get a Grip looks are used.
- Old upscaled `textures/upscaled/racket/racket00_*.png` in Get a Grip mods now holding another racket are unused
  (matched by image name); rerun `tools/upscale_mods.py` to upscale the new ones.
- Court not drawn in the `--shot` frames at 4 s (unrelated to this task; HUD and players are).
