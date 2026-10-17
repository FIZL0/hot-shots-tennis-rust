# B560 — mod characters' hit-by-the-ball cry (voice program 5)

`sound::hit_cry` plays program 5 key 0 when the ball touches a player (`play/bodyhit.rs`). A mod bank plays its
`voice/5_<k>.wav` (key wraps over the takes, so one take is enough); with none it stays silent (B181). No packaged
mod had one. Done in ../opentennis-mods (commit 98d5986, not pushed: the run doesn't push):

- **Get a Grip** (15): its own labelled cue `pc_voice_body` = hit by the ball. faster-whisper on all 15 `sgv_pcNNa`
  takes: "Ow!", "Ouch!", "GAH!", "AY!", "Nah, I'm hit!" (Helghast)… `package.py` GAG_VOICE 5 = `body`, KEYS 5 = 1.
- **Out of Bounds** (17) / **Open Tee 2** (21): golf has no such event and no cue labels, so per actor one short
  English take picked by transcript, in tiers: a pain word ("Ouch!" Fernando/Mika/CJ, "Ow" Kate/Maruyama/Alia),
  else a pained yelp ("Ugh!", "Gah!", "Wah!", "D'ah!", "Yikes!", うっ), else a startled "Ah!"/"Oh!" (Roger: "What?!",
  Shu: "UGH!", hand-picked). Waves several actors share (OOB's common "AHHHH" scream) are never picked. Frozen into
  `oob_data.HIT` / `opentee_data.HIT` (bank/wave, transcript in a comment); `voices()` returns them as program 5.
  The 821 untranscribed source waves (lines already in the mods) were transcribed with the same model as
  `tools/voice_candidates.py` into a scratch cache (not committed, not merged into its cache).
- **Fore!** (24): no pick. Every take is a word line (heckles, comments, versus); the only short exclamations are
  "Oh!" in Regis/Maya/Kamala's jy3/jy0, the happy shot-reaction voices. These stay silent (notes/fore.md).
- `tools/rerig/hit_cry.py MOD_DIR…` adds 5_0.wav + voice_counts/voice_why "5" to packaged mods in place and bumps
  `version` (none → 2, 2 → 3); idempotent. Run over `out/mods/*_pc*`: 53 mods updated (all mono PCM16), 24 Fore! skipped.
  `package.py` now keeps an existing mod.json `version` when it repackages (it used to drop it).
- Standard §5: program 5 row; "a program you ship no wav for stays silent" (it still said the donor plays, pre-B181).
- `sound.rs` hit_cry doc line only; no code change in the game.

## Not verified / not 1:1
- Not uploaded to the mod store: the 53 updated mods are in `opentennis-mods/out/mods` with bumped versions;
  re-uploading is for the human (as B546). Players only get the cries after that.
- Picks are by transcript only, not by ear: whisper's "Ah!"/"Oh!"/kana on a wordless take may be a sigh rather than
  a yelp; the third-tier picks (Jasmine, Sophie, Sonia, Bloom, LJ, Anya, Alex, Gloria (OOB); Patricia, Holly, Luke,
  Alice, Malachy, Frauada, Lee, Gloria (OT2)) are the weakest. Not 1:1 by nature: golf has no hit event.
- Not heard in a match: the playback path (mod bank → program 5 → wav) is the generic one every other mod program
  uses (audio.rs `play_as`); no PCSX2 check needed (in-match presentation, no sim/RNG touch).
- Fore! characters stay silent when hit (no fitting take).
