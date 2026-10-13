# B227 follow-up (user 2026-10-13): no upscale option in the browser, no upscaled download

- Settings: in the browser, Upscaled Textures shows OFF, can't be switched on, and says "Upscaled textures aren't
  available in the browser." (`main_menu.rs NO_UPSCALE`). `textures::set` keeps replacements off there whatever
  settings.txt says, so the texture-replacement packs aren't used either.
- Store install in the browser (`modstore.rs ranged`): no whole-zip download any more. The zip is read part by
  part with HTTP range requests (the store's 302 to its CDN followed once, the rest asked of the CDN URL), and
  `modfs::unpack_with` gets a `keep` filter so the `textures/upscaled/` entries are never fetched. Checked against
  the store: HF answers `Range` with 206 and `Content-Range`, CORS allows it (preflight on both huggingface.co and
  the CDN answers `range`), `Content-Range` is exposed.
- Real run (`ranged_install_leaves_the_upscaled_set_on_the_store`, ignored, network): Anya from PR 2 fetched
  27 MB of her 489 MB zip and kept 30 files, 35 MB, with no upscaled ones.
- B204's OPFS `mod-download.zip` and the page's `hstZip`/`hstZipDrop` are gone; the page deletes a leftover file.

## Not verified / not 1:1
- The browser's whole-zip sha256 check is gone for store installs (it needs every byte). They now check the zip's
  size against the store's and each file's CRC-32, over HTTPS from the same repo that holds `meta.json`.
- Not run in a real browser (Safari/iPhone): the range requests were checked with curl and through reqwest
  natively, not through the browser's fetch. The iPhone check under PLAN.md *Needs the human* covers it.
- The store's listing still shows the whole zip's size in the browser, not what it will download.
