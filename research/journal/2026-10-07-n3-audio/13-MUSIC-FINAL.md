# N3e — music and jingles (FINAL)

Result: `snd::Sequencer` steps a BGM `.mid` a frame at a time like the game's SMF player; `Bank::program` resolves a MIDI note-on to its tones through the bank's program section (0x10); `audio.rs` plays the BGM (`Sound::music`, `Sound::music_volume`) and the jingles (slot 8). Per the user, BGM is off by default: `--music` turns on the court BGM in a `--play` match and `--sound SND/BGM/<X>.XB data/sound/BGM/<Menu|Court>/<x>.hd 0 0` auditions any BGM. Jingles are always on. The note-on timing check against a ring recording was not done; it is left to stretch task M4 with the recording already captured (`context/recordings/bgm_s01.bin`).

## Files
- BGM: `SND/BGM/BGMM_03..10.XB` (menu, `data/sound/BGM/Menu/bgmm_NN.{hd,bd,mid}`) and `BGMG_01..11, 14, 15.XB` (court, `.../Court/`). All 21 `.mid` files are format 0, one track, division 480. They use 0x80/0x90/0xb0/0xc0/0xd0 and metas only, with no sysex and no pitch bend. CCs: 7, 10, 99 (20/30 loop), 6, 0, 32.
- Jingles: `SND/JGL/JIG_00.XB` (`data/sound/JINGLE/jig_00.hd`) in normal matches (game mode ≥ 2), `JIG_01` otherwise. They are SE-style sequences played through the SE wrapper: slot 8, program 0, volume 0x80. Slot 8's bank volume is category 4, which gives 92.

## SMF player (per frame, 60 Hz)
- Slot state holds a double tempo, a division, a tick increment, the current tick T and the next event's tick.
- At start: 120 bpm, inc = 2·div/60·scale/100 (scale 100), T = 0, next = the first delta.
- Each frame: T += inc. While T − next ≥ 0, run an event and add the following delta to next.
- Tempo (ff 51): bpm = 6e7/µs, inc = 1e6/(6e7/bpm)·div/60·scale/100.
- The player handles the loop controllers itself:
  - CC99 = 20 marks the loop start: the position after the CC, plus the running status.
  - CC99 = 30 is the loop end. It decrements the count unless the count is 127 (forever, the default), and jumps back while the count is > 0.
  - CC102 sets the count. CC90 is a marker callback.
- Other CCs and the channel messages go on to the driver.
- Metas: ff 2f ends the track, ff 58 skips 4 bytes, others skip their length. Sysex has a varlen length and goes to the driver; the BGMs have none.

## Driver note-on (BGM port)
- A note plays nothing when the channel's volume, the expression or the velocity is 0.
- Program header byte 0 selects the tone:
  - ff: the tone at note − lowest (byte 6). A note below the lowest plays nothing.
  - Otherwise: scan (byte & 0x7f) + 1 tones whose key range (tone bytes 0..1) holds the note. Without 0x80 only the first matching tone plays; with 0x80 every matching tone plays (layered).
  - A tone with byte 0xd = ff ends the scan.
- Program volume is header byte 1.
- Tone level = channel volume (CC7, reset 100) · expression (CC11, reset 127) · program volume · tone volume / 127³.
- Channel pan (CC10, reset 0x40) is clamped 1..127. Tone pan is as for SE.
- Sequence volume is the port's L/R, which the game sets to its BGM volume of 55.
- The bank term is the voice default 127. Only SE ports get the bank volume.
- Velocity is raw (no velocity table on the port).
- Port cmd 4 (master balance via universal sysex `7f 7d 04 02`) sets a voice field that is not a volume term.

## Game side (director)
- Court BGM is bgmg_NN for court NN (1..11) in a normal match, and bgmg_14 otherwise. It starts at volume 55 from the top on director message 6.
- Message 0x19 (end of a game or a set):
  - Plays jingle key 1 for a game or key 2 for a set.
  - Fades the BGM down by 55/60 a frame to 0.
  - Fades it back up by 55/60 a frame once the jingle has ended and message 0xe or 6 clears the hold.
- Match end plays key 3 when the local player's side won and key 4 otherwise. It does not fade.
- Message 0xc plays key 0. Its meaning is unknown; it may be the change of ends. hits_s04 has key 0 148 frames after key 1.
- In the port: the jingle plays at the point's verdict and the hold lifts at the next point. Key 0 is not wired.

## Left (stretch M4)
- Note-on timing against the ring. `bgm_s01.bin` is 1800 frames from save slot 1 (bgmm_05 on SMF slot 5, 269 frames missed at full speed). Each frame is a u32 vsync, 8 SMF slots × 0x80 (T at +0x48, next at +0x50, track pointer at +0x60), a u32 n and n 16-byte ring commands. Key-on masks are cmds 7/8/9 and the pitch word is cmd 4 (note in bits 16..23). The recorder script is beside it.
- Live CC refresh of sounding voices.
- Reverb (CC6 data entry).
- The director's restart and hold cues instead of the point-based approximation.
- Menu BGM inside a menu.
