# P44 — frame pacing at high Hz (2026-10-15)

## What changed
- `HST_FPS=1` log (`main.rs` `frame_spread`, after each `fps` line): every 5 s the frame-time spread: mean, sd,
  p50/p90/p99/p99.9, max, **jitter** (mean |dtₙ − dtₙ₋₁|: pacing, 0 for an even run) and the **1 % / 0.1 % lows**
  (fps of the slowest 1 % / 0.1 % of frames, averaged). `tools/bench.sh` prints these `frames ms:` lines too. The
  dt is Bevy's `Time<Real>`, which the render world stamps right after its present: the present-to-present interval.
- `HST_PRESENT=fifo|mailbox|immediate` (`graphics.rs` `present_mode`): one present mode for a bench, over Vsync.
- Frame Limit choices: 30, 60, 120, 144, **240, 360, 480**, Uncapped (`graphics::FPS`; menu row cycles through it).
- **The frame limiter now sleeps in the render world after the submit, right before the present** (`limit_present`,
  in `RenderGraphSystems::Finish`). The old one slept at the main world's frame end, which paced when rendering
  *started*. The render world's time then varied from frame to frame, so the presents were spread unevenly. `HST_PACE=main` brings the old one
  back. Show FPS → Detailed counts the sleep as idle, not submit.

## Measurements (1440p window `HST_WINDOW='2560 1440'`, 4-bot doubles court 1, RX 9070 XT, load average 18–36 from other agents)
Logs: `context/perf/p44/`. Lines are the steady 5-s windows.

| run | mean ms | sd | p99 | jitter | 1 % low |
|---|---:|---:|---:|---:|---:|
| uncapped (AutoNoVsync) | 3.8–4.5 | 1.1–1.2 | 8–9 | 0.7–0.9 | 88–112 |
| Mailbox | 3.6–4.5 | 0.9–2.2 | 7–14 | 0.7–1.0 | 63–110 |
| Immediate | 3.7 | 0.7 | 5.6–6.3 | 0.65 | 132–147 |
| cap 60, old limiter (main) | 16.67 | 1.1–2.4 | 20–23 | 1.4–3.0 | 37–48 |
| **cap 60, new (render, before present)** | 16.67 | **0.38–0.69** | 18–19 | **0.29–0.66** | 50–54 |
| cap 120, old | 8.4–8.6 | 1.7–1.9 | 14–15 | 1.8–2.2 | 58–65 |
| cap 120, new | 8.4–8.6 | 1.0–2.1 | 12–17 | 0.8–1.2 | 45–69 |
| cap 240 / 480 (new) | 7.1 / 8.4 | 1.7–1.9 | 12–15 | 1.4–1.7 | 62–70 |

- Uncapped at 1440p runs ~220–270 fps when the load is lighter, and ~130–150 when other agents' builds peak (the
  capped and uncapped A/B runs taken one after the other came out the same, 6.6–7.6 ms). The machine is CPU-bound by its
  neighbours, so 240/480 couldn't be reached and their own pacing couldn't be judged here.
- A first cap-240 run looked slower than uncapped. That was load noise: `sim` was 3× slower in that run, and the
  A/B runs taken one after the other match. Sleep overshoot on this box: p50 0.05 ms, p99 0.24 ms
  (2000 × `sleep(2 ms)`), so no spin is needed.
- Fifo (vsync) ran at exactly 60 fps with `acquire` 10–12 ms on a 240 Hz monitor. The bench window sits lowered
  behind others on workspace 1, and the compositor paces a hidden surface at 60. This is not a game limit, but it can't be checked unattended.

## Not verified / not 1:1
- 240/360/480 Hz smoothness at 1440p: not reached. Other agents loaded the machine (load 18–36 on 16 threads)
  and held it at ~130–270 fps. The tools are in place; a rerun on an idle machine is part of P35's re-bench and is in `plan/HUMAN.md`.
- Vsync/Fifo at 240 Hz: the unattended bench window is occluded, so the compositor gave it 60 Hz. It needs a visible
  window (HUMAN.md).
- Input lag of the new limiter: not measured. As with vsync, the main world now waits for the render world, so up to
  one frame more lag than the old limiter at a cap (≈4 ms at 240, ≈17 ms at 60). `HST_PACE=main` restores the old one.
  No sim/RNG/match-timing change: the sim's 60 Hz fixed step is driven by `Time<Real>` as before.
- Not run on iOS/Android/wasm. wasm skips the limiter as before. On iOS/Android the sleep now holds the frame's
  drawable until its present, which is not checked on a device.
