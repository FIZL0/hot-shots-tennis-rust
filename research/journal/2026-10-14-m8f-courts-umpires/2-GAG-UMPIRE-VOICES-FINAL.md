# 2 — Get a Grip umpire voices as umpire mods (M8f2)

## Cue → wave, decoded
Get a Grip's SGXD banks name cues (NAME ids 0x2000cccc), not waves. Cue c plays SEQD group 0 sequence c; each
sequence keys on with `d0 01 NN`, where NN is the note **in decimal digits** (0x32 = note 32, 0x40 = note 40): across
all 14k Get a Grip sequences the digit reading always lands on an RGND region's key range, the raw byte often
doesn't. The region holding the note gives the wave. ../opentennis-mods `tools/audio.py sgxd_cue_waves`.
- Umpire banks `gag_vcNN{a,b}`: 45 cues, 45 regions, cue c → note 32+c → region c → wave c (identity).
- The old inferred rule (cue c → region c) agreed for the playable `sgv/dgv_PC00-13` banks, was one off from cue 5 on
  in 44 of 46 `dgv_NPC_*` banks (checked by transcript on `dgv_NPC_m06a`: decoded `start_0` "Shall we?", `match`
  "Last one!", `go` "Now!"; the old rule gave the neighbours) and named nothing in `sgv/dgv_PC14` (Helghast).
  New task M92: repackage pc14 with its voices.

## Who is who
Umpire NN: bank `sound/game/010_umpire_NN_0V.xb`, model `200_court/umpire.xb` u_NN (face textures Natashya, Ryota,
Vanessa, Javier, Yakun, Tenibo, Suzuki, Gloria), loading card `menu/loadmes/judge/loadmes_jNN.xb0` (English):
00 Natasha (f, 19), 01 Nate (m), 02 Kanani (f), 03 Xavier (m), 04 Helghast (m, gas mask), 05 Robo T. (robot),
06 Suzuki (m), 07 Gloria (f). Bank 04's takes come out filtered ("Move!" for love), 05's garbled, as a mask/robot.

## Transcripts (faster-whisper small, English, all 16 `nml` banks, 720 clips)
Every cue says its name in all 16 banks (one-syllable numbers mis-heard as "Bye", "Touchy", "That's it" for 5 / 30 —
same words in the other banks). HST's five disc banks (`bankwav`, variant a = the one a match plays): 0_11 "server",
0_12 "receiver", 2_3 "change", 2_4 "game set and match"; no take for 0_13 in any of them.

| HST call | Get a Grip cue |
|---|---|
| 0_0–3 / 0_4–7 | `vc_voice_point_f_0-3` / `point_b_0-3` |
| 0_8 all, 0_9 deuce, 0_10 advantage, 0_11 server, 0_12 receiver, 0_13 deuce again | `point_all`, `point_deuce_0`, `point_adv_0`, `point_adv_1`, `point_adv_2`, `point_deuce_1` |
| 1_0–4 out, fault, double fault, let, net | `miss_out`, `miss_fault_0`, `miss_fault_1`, `miss_let`, `miss_net` |
| 2_0–4 tiebreak, game, set, change, match | `break`, `game`, `set`, `change` ("change sides"), `gameset` |

Unused: `ballhit` (no HST call), `tiebreak_Nup/down` (HST calls no tiebreak points, `call_score`), the `b` take set
(chattier, "That's out!"), the `_rev` copies.

## Packaged
../opentennis-mods `tools/rerig/gag_umpires.py [HST dir]` → `out/mods/gag_umpNN_<name>/` (8 mods, `kind: umpire`, 24 calls
each, `calls`, `calls_why` cue per call). Donor: the same umpire where HST has one (Suzuki 1, Robot Tennis 3 — both
also the least-overrun fit), else the same-sex HST umpire whose takes the clips overrun least: Natasha, Kanani,
Gloria → Lily; Nate, Xavier, Helghast → Suzuki. `todo` lists calls > 0.1 s longer than the donor's (2_3 "change sides"
and 2_4 for nearly all; Nate and Helghast more). Standard §3a: rows 0_11/0_12/0_13/2_3 reworded from the transcripts.

## Not verified / not 1:1
- Not heard in a match: no loader yet (M8f3). Whether a mod's 0_13 plays (the donor has no take, so no length to
  time it by) and how a clip longer than the donor's is cut are M8f3's to decide.
- Voice index NN = model u_NN = card jNN is by order (all three lists run 00–07 in the same order); supported by
  04's masked and 05's robotic takes and by Suzuki/Robot Tennis fitting their HST namesakes' timing best, not by
  a table in the data.
- Sex of the GaG umpires from the loading cards' pictures/text, HST's from `hst_umpires.py`.
- Transcripts give words, not tone; nothing was listened to by ear.
- Front-end/presentation: no bit-exact claims apply (voices only replace what is heard; timing stays the donor's).
