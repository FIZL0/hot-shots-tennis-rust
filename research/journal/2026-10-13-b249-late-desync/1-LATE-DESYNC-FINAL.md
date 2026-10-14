# B249: rollback_desync_is_saved_and_play_goes_on after B231

Instrumented `netplay::desync`: the host's ggrs reported the forced desync (tick 330) first, sent its state, and
the guest took it (in sync from tick 338) *before* the guest's own ggrs raised `DesyncDetected` for tick 330. That
late report was `resync.stale` and returned early, so the guest never noted or saved the desync → `reported()`'s
"the desync's note" panic. Nothing in B231's `play::motions` change is wrong; which peer's checksum comparison
fires first depends on ggrs' message timing (one instrumented run passed), and B231 shifted it.

Fix (`netplay.rs` `desync`): a report for a desync a resync already mended is still noted and its inputs saved once
(when this machine has no note yet), without asking for another resync or marking the machine out of sync.
Test passes on repeated runs.

## Not verified / not 1:1
- none (netplay is ours, no original to match).
