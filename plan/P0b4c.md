# P0b4c

- [ ] **P0b4c — Umpire-call timing.** Faults/outs/lets/double faults: scoreboard call show (`388190`, 0x427 →
  chained score show with a 1-tick pause for out / out-after-net / double fault) ends when the call sprite
  animation is done and the umpire voice has stopped (fallback countdown table 0x410f0c by call × language);
  recorded faults take 79 or 80 ticks (voice jitter). Also the match-over wait (`326270`: voice line, then
  phase 5). Needs the sprite anim length from disc and the voice clip lengths (P13/P20).
