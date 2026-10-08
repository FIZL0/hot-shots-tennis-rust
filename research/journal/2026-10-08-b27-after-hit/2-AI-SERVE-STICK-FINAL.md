# B27b — the computer server's stick through its follow-through (FINAL)

- **Capture**: `research/b27b/serve_ai_step.py` replays `new_recording.p2m2` lock-step on a copy (launched
  `HST_REALTIME=1`; one PINE connection; pad bytes match the recording at d=1) and records, per frame, the globals
  +0x423048.. and per player its AI's +0x50.. / +0x248 and its +0x3d70.., +0x3e50.., +0x3f00.., +0x3f94... It drifts
  from the recorded game after a while (contacts land at other frames) but is a genuine run of the original. Kept as
  `context/fixtures/serve_ai.bin` (vsync 5863..10400, 4532 frames). Wall-clock replay at 0.25× or 0.5× diverged
  within seconds (input lag jitter, missed vsyncs).
- **Original**, from the update dispatch and the BASE/NET rally routines, confirmed in the capture:
  - The AI stays in its serve state (top state 1) until the tick after the stroke flag +0x3f95 clears, i.e. the
    frame after +0x3f00 reaches the recovery. It then goes to rally (state 3, sub-state 0).
  - Rally sub-state 0 returns with no stick while the shot count (0x423060) is below 2. So an unreturned serve's
    swing always plays out.
  - The reaction frames (+0x248) are drawn on the return's hit message, after that frame's AI tick. The capture shows
    6 (vsync 8793), 4 and 3 on the return frame. From the next frame, sub-state 0 decrements them, one per frame,
    with no stick.
  - With them at 0 the move logic runs: sub-state 1 and the position moves at once (8800), or a frame later (10339 x
    moves, 10340 sub-state 1).
  - Both capture serves returned before 54 played out with frames still held (hold 1 at play-out). The recordings'
    break-offs (round1 p3 at 47–53, new_recording p1 at 50–55) are all ≥ 4 frames after the return.
- **Port**: `hst_sim::motion::ai_serve_stick(after, recover, shots, fresh, &mut hold)`. play.rs `ai_serve_stick`
  feeds it the served frames, `ai_hold` and `g.flight.frame == 0` for the return's own frame. That covers the port
  striking before or after the bot in the same frame: the hold isn't counted on the frame it is drawn. `bot` passes
  the result to `serve_follow` as the stick.
  ponytail: once the wait is over the stand-in always breaks off. The original's move step can give no stick
  (within ⅔ of a step) or start a frame later (B27c).
- **Test** `player.rs ai_serve_stick`:
  - Over serve_ai.bin (428 frames, 22 counting down, 5 moves): the hold matches frame by frame, and the AI never
    moves while the port says wait. It moves within a frame once the port says go.
  - round1/new_recording computer servers: none is off before the return plus a frame (10 returned cases). Ends
    within a frame of the player's longest unreturned play-out count as played out.
