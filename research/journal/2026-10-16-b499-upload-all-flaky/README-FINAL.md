# B499 — mods_screen::tests::upload_all flaky under the full suite

Cause: `modstore`'s events are one process-wide queue (`modstore::take`). The test ran `page` to apply its upload
outcomes, and `page` applies every event in the queue. Other tests in the suite that open Browse or the main menu's
store column call `modstore::list` (a cached listing is sent at once, else a real read), so a `Listing` could land in
the same frame as, or after, the last `Upload(.., Failed)` and replace "Not uploaded: …" with "N mods in the store.".
`PAGE` only serialises tests that run `page`, not the ones that start store reads.

Fix: the test drains the queue itself and applies only `Event::Upload` for its own two mods through
`View::uploaded` (exactly `page`'s handler for that event); other tests' events are ignored. Still holds `PAGE` so a
`page`-running test can't eat its outcomes. Game code unchanged.

Verified: `tools/check.sh -p hst upload_all` passes; the full suite runs in done.sh.

## Not verified / not 1:1
- none (test-only change, no gameplay or rendering involved).
