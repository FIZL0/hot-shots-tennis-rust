# B509 — Get a Grip court ambience: nonstop, "a short sample repeated" (Sunflower Village)

## What the remaster plays (checked)
- A headless bot match on Sunflower Village (`gag_c03_denen`, layout 0 → court 1) for 3 minutes (4 points and a
  change of ends), the mixer rendered every tick with only the ambience volume group up: the output is
  `ambience_0.wav` itself (resampled 44.1 → 48 kHz, × its `AMBIENCE_RMS` gain 0.246), correlation 1.000 second by
  second up to the loop's wrap, nothing else in the group. Court 1's program 7 (the layout court's creature sounds)
  is silent, and the stream is never restarted or held between points, in the post-point timeline or at the change
  of ends. The game plays the file as it is.
- `ambience_0.wav` is GaG's own decode: a fresh `vgmstream-cli` decode of `env_co03.sgh/.sgb` (one subsong, ATRAC3
  stereo 44.1 kHz, loop 308631..5102919 = 7.0 s to the end, 115.7 s) matches it sample for sample (max diff 0).
- What it sounds like: about 55% of its energy is a narrow band at 4–5 kHz (plus ~4% at 3 kHz) over low wind/rumble
  under 500 Hz, the band's envelope pulsing at ~2 Hz: an insect chorus (crickets/cicadas) over a breeze. No repeat at
  any lag from 20 ms to 2 s (waveform autocorrelation ≤ 0.13, envelope autocorrelation decays smoothly), no gaps
  (0.2% of 50 ms windows under a quarter of the median level). The "very short sample repeated" is the chorus's own
  chirping: the content of Get a Grip's stream, not a short clip or a looping fault.
- "Nonstop": `env_coNN` is one looping ATRAC3 track (loop points in its `.sgh`; on the title GaG starts its streams
  with `sceAtracSetLoopNum(id, -1)`, endless); with nothing on the PSP showing a gap, nonstop stays.

## Change
- `court_mod_match.rs`: the court-mod match test now renders the mix each tick with only ambience up and asserts it
  equals the mod's loops played alone (`Sound::bare` + `play_loop`), so program 7 leaking through or a loop being
  restarted/held fails it (checked: with program 7 not silenced it fails at tick 658). `Sound::render_frame`
  (testing only) renders one frame of the mix.
- `research/ppsspp_ws.py`: a no-dependency client for PPSSPP's websocket debugger, for driving GaG (see below).

## Not verified / not 1:1
- Whether Get a Grip itself ever pauses, ducks or restarts the ambience (between points, during cut-aways/replays):
  not checked on the PSP. `PPSSPPHeadless` boots GaG and logs its sound calls (`-l`: `sceAtrac*`, `sceIoOpen`), but
  with no input it stays on the title (no attract match in 280 s). Its `--debugger=PORT` and the SDL build's remote
  debugger (`RemoteDebuggerOnStartup`, run with `SDL_VIDEODRIVER=offscreen` or under `gamescope --backend headless`)
  both accept the TCP connection but never answer an HTTP request (each connection's handler thread waits on a
  futex), with and without the sandbox; the offscreen SDL run also froze at the intro movie after 85 s. Didn't put
  a PPSSPP window on the desktop for an unattended run. Asked the human to listen (`plan/HUMAN.md`). If it does gap,
  the change is in `court_mod.rs` `sounds` (stop/restart the `play_loop` ids on the match's phase).
- How loud GaG mixes the ambience against its BGM: not measured; the remaster's level (`AMBIENCE_RMS`, ~10 dB under
  the BGM) is M111's stock-court level.
