# B134 — pop-ups when the ball hits a wall or object

## Findings
- The original's pop-up for a wall/object hit is the same "bounce word" the court and net use (B85, P39c): the
  rally pop-up object makes a word on each new ball contact (fewer than 6 contacts so far), if the 4-frame cooldown is
  over, fewer than 10 words are alive and the rally flag is set. The texture is 12 + the material's effect kind
  (court 10's ruins: cloth-hung wall → CLOTH, wooden props → WOOD, floor → YUKA, dirt → TUCHI). Its sound is the
  material's sound key, played only while the rally's sound flag is set (cleared by the point-end messages).
- All of that was already ported (`hst-sim/src/bodyhit.rs` `BounceWords`, `hst/src/play/bodyhit.rs` `bounce_words`
  /`draw_words`, bounce sounds in `play.rs` simulate). Two new recordings showed the words appear on the right frame
  with the right texture, but the position was off in the last bit for a prop/wall contact.
- Fix: the contact record's point is the ball's placed position for the hit (`contact::contact_point`, plus 0.0)
  less normal × radius, not the hit's centre less normal × radius (`ball.rs` `Flight::step_world`). Net cases (B85)
  are unchanged by this; cloth and wood now match bit for bit.

## Recordings (context/b134/, git-ignored)
- `probe.bin`, `cloth.bin`, `wood.bin`: court 10 rallies from slot 5 with the ball poked to fly into the ruins'
  cloth wall / a wooden prop as its first contact, recorded like B85's (`tools/record_live.py`-style word capture).
- Screenshots `cloth/orig_*.png`, `wood/orig_*.png`.
- Test: `crates/hst-sim/tests/live.rs` `net_touch_words_match_the_game` now also runs `b134/cloth` and `b134/wood`
  (skipped when the recordings are absent).

## Not verified / not 1:1
- Visual side by side: the original cuts to the post-point cut-away right after a wall hit, so its screenshots show
  the cut-away, not the word; the word's texture, position and timing are checked from RAM instead.
- (B85's four net recordings, copied from slot s5's context/, also still match bit for bit with the new point.)
- The rolling "kusa" word (from the ball's rolling flag, not a contact) isn't ported; no court-10 wall hit triggers
  it. Not opened as a task here; it belongs with grass courts.
- Only court 10's cloth and wood were recorded; other courts' wall materials (metal, water, ...) use the same path
  and table but weren't recorded.
