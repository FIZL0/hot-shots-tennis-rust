# B567 iOS 1.0 IPA doesn't offer 1.1.0 — FINAL

Checked: the release side is fine (v1.1.0 is "latest", has the IPA); the shipped v1.0.0 IPA's binary has
`HST_VERSION=v1.0.0` and the releases/latest check; reqwest on iOS verifies through rustls-platform-verifier
(Security.framework), and the iOS notice only needs a found tag (no asset: Download opens the release page).

Cause: the check runs at launch, then `thread::sleep(30 min)`. 1.0.0 went out 00:38, 1.1.0 07:45; iOS resumes a
suspended app instead of relaunching it, and a sleeping phone's suspended time doesn't count, so the half hour
can be hours and an app opened before 1.1.0 never asks again.

Fix: the checker parks (`park_timeout`) instead of sleeping; `update::recheck` (Last, every platform) unparks it on
`AppLifecycle::WillResume`. Not verified on a device (no iPhone here); `hst-log.txt` in Files → OpenTennis shows
an `update check:` line if the request itself fails.

Follow-up (user's hst-log.txt, 2026-10-17 17:00): the phone was already on the v1.1.0 release IPA ("Hot Shots
Tennis 1.1.0 (build 3238)"; the published 1.0.0 IPA is build 3079, 1.1.0's is 3238), so there was nothing newer to
offer, and the log has no `update check:` error. The resume re-check stays: it's still right for an app left open.
