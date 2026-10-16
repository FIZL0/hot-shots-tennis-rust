# M114 — uploads in the Mods footer's status

The footer's status (B471, `grid::status`) now covers uploads, on every Mods page:

- uploads queued or under way come before the last action's words: spinner + `Uploading <name> · N queued`
  (`Uploading <name>` alone; `N uploading` before the first has started);
- downloads first when both run: `2 downloading · 1 queued · 3 uploading`;
- once the last upload is done, `Uploaded N mods · waiting for review` for 4 s (`UPPED_SECS`, counted down in the
  page system on `Time`), then back to `N installed` (or an author's list's `… by …`).

Details:
- `View::upped` = (uploads that went up since none were under way, seconds left). `queue` starts a new count when
  nothing is uploading and no longer writes "Uploading … in the background" into `status` (the footer says it).
- The `Event::Upload` arm moved into `View::uploaded`: a Done no longer puts its PR message in `status` (it
  counts instead); Skipped / Failed still do, so a failure is what the footer says after the uploads (and after the
  4 s window if some went up).
- The footer draws its status with `frame::dotted` (the font has no `·`); the downloads' old `-` separator is a `·` too.
- `--shot` hook: `HST_UPS=1` (first installed mod uploading, next two queued), `=2` (plus two downloads), `=done`.
  Shots in `context/m114/run1/` (Browse with downloads + uploads, Upload, Installed with the done words).
- Test: `mods_screen::tests::upload_status`; `upload_all` updated.

## Not verified / not 1:1
- The concept card (claude.ai artifact) couldn't be opened from here: the wording follows the task line; the pill is
  the existing footer status + spinner (no separate pill shape).
- No real Hugging Face upload was run (needs a token/network); the event flow is covered by the unit test and the
  existing `upload_all` packing-failure test.
- Front-end only: no original-game comparison applies (PLAN: menus are ours).
