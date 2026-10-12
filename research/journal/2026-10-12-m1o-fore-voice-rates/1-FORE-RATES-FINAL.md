# M1o: Fore mod voices too high pitched (FINAL)

## Fix (HST-MODS)
- 106a626 `tools/audio.py`: each Fore SShd sample is written at its true rate 44100 · 2^((note − root + fine/16)/12),
  from the tone record (root +2, fine i8 +3, sample address/8 +4, the `snd.rs` layout). Fore's driver
  (`MODULES2/SNDN2DRV.IRX`) uses the same 192-steps-per-octave pitch rule as HST's (`snd::pitch`, N3a), so the tone's
  own note keyed at scale 0x1000 is what the game plays. `fore_rates` rewrote every already-converted wav header in place
  (6081 banks, all `index.txt` now carry `semitones= rate=`).
- The mods' `voice/*.wav` are hard links into `out/audio`, so `out/mods/fore_*` already carry the new rates (Sam's: 26033 /
  30078 Hz, his tones at −6.625 (139), −13.25, −9.125, −7.625 st). Nothing else to repackage: HST reads `$HST_MODS` in place.
- Only 4 voice samples in all of Fore have tones at two pitches, all gallery crowd (`gaya/ga_03/07/09/12`); first tone wins.

## Proof against the running game
`research/tools/fore_spu_rates.py STATE.p2s…`: for every sounding SPU2 voice in a PCSX2 save state of Fore, finds the
sample (64 bytes of SPU RAM at the voice's start address vs every tone's .bd bytes) and prints the SPU pitch register as a
rate (pitch · 48000 / 4096) next to audio.py's rate. Fore was booted on copy 4 with a scratch copy of `pcsx2-hst.sh`
pointed at the Fore ISO; ~100 states saved over its attract demo (4-player golf).
- Brad (pc05) in-game shout `PC/PC05/PC050002.XB/.../g_bra2.hd`: game 29004 Hz, audio.py 29011 Hz (−7.25 st).
- Caddie Kaz: −7.8125 st 28078/28084, −9.8125 st 25008/25020, −10.875 st 23520/23530 (several banks).
- Differences are the pitch table's integer rounding (< 0.05 %). SE banks (`plse`) also play at other notes
  (sequenced), as expected; voices always at their own note.

## Open Tee / Open Tee 2 / Out of Bounds
`rates.txt` exists for every SGXD bank with an RGND table (GaG's rule, user-confirmed by ear). Every bank without one was
checked: none has RGND. OT2's `pc_g_NN[ab].sgd` (in-game shouts) stay at 44.1 kHz — they are stereo ATRAC3 RIFFs
(fmt says 44100), which the PSP plays at their own rate, so that is right.

## Not verified / not 1:1
- Nothing listened to by ear (unattended run, PCSX2 copies are muted): Fore checked by SPU pitch register instead,
  OT/OT2/OOB by their headers only (no PPSSPP/RPCS3 run).
- Sam himself never spoke in the sampled demo; his banks use the same rule as Brad's, verified.
- Fore SE wavs are written at their own-note rate; SE the game sequences at other notes are not reproduced (not voices).
