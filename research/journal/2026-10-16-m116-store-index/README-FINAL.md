# M116 — "Can't reach the mod store": a store index

## 1. Proof: HF rate limit
A Python replay of `modstore::list`'s crawl (anonymous, every request's status logged; logs in `context/M116/`):

- First run: the very first two requests already answered **429** with `RateLimit-Policy: "fixed window";"api";q=500;w=300`,
  `Retry-After: 99`, `X-Error-Message: We had to rate limit your IP …`.
- After the window reset: **82 open PRs, 168 requests per listing**: 86 in the `api` bucket (3 discussion pages, a
  tree per rev) + 82 `resolve` meta.json reads (`resolvers` bucket, 3000 / 5 min). `main` has no `mods/` yet (404).
  So ~5 listings in 5 minutes from one IP (several agents/players behind one, M115 re-listing per upload, `uploads_of`
  per upload batch) exhaust the API bucket, and every answer after that is 429 → "Can't reach the mod store."

## 2. The index
`../opentennis-mods/tools/store_index/` (pushed, 75cfde0):
- `build.py`: lists refs (PR head commits), open PRs (author), `mods/` trees, meta.json; writes `index.json`
  (`version`, `built`, `pulls` {n: author, commit}, `entries` [meta, rev, verified, folder oid]) to the dataset's
  `main` only when pulls/entries changed. Incremental: an unchanged PR head or main folder oid is reused from the
  last index (a no-change rebuild ≈ 4 API calls). First index committed from here (82 mods, 82 PRs); a second run
  said "unchanged".
- `space/` (Docker FastAPI: `POST /webhook`, checks `X-Webhook-Secret`, rebuild worker coalescing bursts, rebuild
  on start) + `deploy.py` (creates the Space `FIZL0/opentennis-mods-index`, its secrets, and the dataset webhook,
  domains repo + discussion).
- `.github/workflows/store-index.yml`: hourly (+ manual) backstop, secret `HF_TOKEN`.

Client (`crates/hst-base/src/modstore.rs`): `lists()` = `index()` ‖ `pulls()`; `parts()` takes the index's
entries for `main` and each PR still open (re-checked by `meta_of`, author = the PR's author from the live list),
`Read`s only PRs not in the index (a fresh upload shows at once), drops PRs closed since. No/invalid index →
the old full crawl. `pulls()` stops at the reply's `count` (50 per page), so 82 PRs = 2 pages.
**Live: a cold listing = 3 requests with 82 open PRs** (`HST_LIVE=1 tools/check.sh -p hst-base live_listing_cost`).

## 3. Busy
`answer()` turns 429/5xx into `BUSY` ("The mod store is busy, try again in a minute."). A failed listing with a
cached one (any age) sends the cached `Listing(Ok)` then `Event::Note(BUSY)` (status line); uploads/removals no
longer blank the cache, they mark it stale (`stale()`; a removal drops that entry). Errors in a crawl now
propagate instead of silently listing fewer mods.

## Not verified / not 1:1
- Webhook → index within ~1 min: not live yet. Deploying the Space and the Action's secret needs a write token in
  two secret stores; left to the human (plan/HUMAN.md) rather than copying the user's own full token there.
  Until then the index only changes when someone runs `build.py` (new PRs still show, read directly).
- A fresh upload showing before the next index run: covered by the `index_then_new_pulls` unit test (a PR not in
  the index is read from the store), not by a live upload (would open a junk PR on the public store).
- Between an index and its rebuild: a commit onto an existing PR (Plan::Onto) shows the old meta (an install then
  fails the sha256 check), and a merged PR's mod disappears until the rebuild lists it on main. With the webhook
  that's ≤ ~1 min; without it ≤ 1 h.
- Menus: no 1:1 needed (front-end).
