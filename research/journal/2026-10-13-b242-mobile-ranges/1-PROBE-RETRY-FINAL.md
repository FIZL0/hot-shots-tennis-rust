# B242 — mobile store installs: "the store sent the whole zip"

## What the store does (2026-10-13, Anya = PR 2, Wendy = PR 3, Z = PR 4)
- `huggingface.co/.../resolve/refs%2Fpr%2FN/mods/<id>/mod.zip` → 302 (`cache-control: no-store`) to
  `us.aws.cdn.hf.co/xet-bridge-us/...` (signed, `Expires` a fixed day boundary, so the URL is stable for a day per UA).
- The CDN answers every `Range` with 206, whatever the User-Agent (curl with iPhone / Android Chrome UAs), including
  `bytes=0-<len-1>`. CORS: HF preflight allows `range`; the CDN allows `*`, exposes `*`.
- In a real browser, same probe + direct CDN part from a page on another origin: Chromium (Playwright, Pixel 7
  emulation) and WebKitGTK 2.x (desktop and iPhone UA) all get 206 for the redirected 0-0 probe and the CDN part.
  Playwright's own WebKit build wouldn't start on this host (missing Ubuntu libs).
- So nothing here reproduces the 200; the server side is fine. Left: the phone's engine (iOS CFNetwork / Android
  Chrome's network stack) dropping `Range` on the cross-origin redirect, or its HTTP cache answering a range with a
  whole zip it kept (the CDN URL is stable for a day, so an earlier whole-zip download can be cached).

## Change (`modstore.rs` `ranged`/`ask`/`partial`)
- Range requests in the browser use `cache: no-store` (no header added, so no new CORS preflight).
- The 0-0 probe: if it isn't a 206 and the fetch was redirected, the response is dropped (aborts the zip) and the
  probe is asked again of the CDN URL directly (no redirect); later parts already go straight to the CDN.
- The error now names the request and where the 200 came from: `(the size check: 200 from us.aws.cdn.hf.co)` or
  `(the part at N: ...)`, so a phone report says which one it is. Still no whole-zip fallback (B227).
- `ranged_install_leaves_the_upscaled_set_on_the_store` (network, ignored) passes; wasm32 builds.

## Not verified / not 1:1
- On an iPhone and Android Chrome: not run (no phone here). If it still fails, the message's `(… : status from host)`
  says which request; added to PLAN.md *Needs the human*.
