// Offline play and an instant start (B470): the page, its scripts and the game are kept in the browser's cache, one
// cache per build (`hst-<version.txt>`), and served from it without waiting on the network. A newer build is
// downloaded whole in the background and only offered once it is complete: index.html polls `version.txt?next`
// (B317), the game's main menu shows Refresh, and the reload (any navigation) switches to it. The disc pack is in
// OPFS, never in these caches: nothing from the disc is hosted.
const META = 'hst-meta';
const SHELL = ['./', 'site.js', 'site.css', 'logo.svg', 'touch.js', 'worker.js', 'opfs.js', 'manifest.webmanifest', 'icon.svg', 'icon-180.png', 'icon-192.png', 'icon-512.png'];
const scope = new URL(self.registration.scope);
const local = path => new URL(path, scope).href;

// the build each cache name holds: `active` (served), `previous` (kept for tabs still loading it), `ready` (complete,
// switched to on the next navigation)
const get = async key => {
  const r = await (await caches.open(META)).match(local(`.meta/${key}`));
  return r ? r.text() : null;
};
const set = async (key, v) => {
  const m = await caches.open(META);
  if (v == null) await m.delete(local(`.meta/${key}`));
  else await m.put(local(`.meta/${key}`), new Response(v));
};
const name = v => `hst-${v}`;
const valid = v => /^[0-9a-z]{1,40}$/.test(v);
// the deployed build: version.txt straight from the server, null offline (or without one: `tools/web.sh` writes it)
const latest = () => fetch(local('version.txt'), { cache: 'no-store' })
  .then(r => r.ok ? r.text() : null).then(t => t && valid(t.trim()) ? t.trim() : null, () => null);
const network = req => fetch(new Request(req, { cache: 'no-cache' }));
const plain = req => { const u = new URL(req.url); u.search = ''; return u.href; };

// every cache but the ones in use (the old single `hst` one, superseded builds, a half-downloaded one) goes
async function prune(keep) {
  for (const k of await caches.keys()) if (k !== META && !keep.includes(k)) await caches.delete(k);
}

// The deployed build into its own cache, all its files (the shell and whatever the current build has cached: the
// renderer's pkg/ or pkg-webgpu/, pictures) before it counts as ready, and version.txt unchanged across the
// download so hst.js and its wasm are always one build's pair. One at a time; a failed one is retried on the next poll.
let staging = null;
function stage() {
  return staging ??= (async () => {
    const v = await latest();
    const active = await get('active');
    if (!v || v === active || v === await get('ready')) return;
    const c = await caches.open(name(v));
    const old = active ? await (await caches.open(name(active))).keys() : [];
    const urls = new Set([...SHELL.map(local), ...old.map(plain)]);
    urls.delete(local('version.txt'));
    for (const u of urls) {
      if (await c.match(u)) continue;
      const r = await fetch(u, { cache: 'no-cache' });
      if (!r.ok) { if (SHELL.map(local).includes(u)) throw new Error(`${u}: HTTP ${r.status}`); continue; }
      await c.put(u, r);
    }
    if (await latest() !== v) return;
    await c.put(local('version.txt'), new Response(v));
    await set('ready', v);
    await prune([name(v), active && name(active), await get('previous') && name(await get('previous'))].filter(Boolean));
  })().catch(e => console.warn('hst: background update', e)).finally(() => { staging = null; });
}

// the first build: the shell now (the game's pkg is cached as the page loads it)
async function first() {
  if (await get('active')) return;
  const v = await latest() ?? 'dev';
  const c = await caches.open(name(v));
  await c.addAll(SHELL);
  await c.put(local('version.txt'), new Response(v));
  await set('active', v);
  await prune([name(v)]);
}

// a reload or a new visit opens the ready build
async function promote() {
  const ready = await get('ready');
  if (!ready) return;
  const active = await get('active');
  if (ready !== active) {
    await set('previous', active);
    await set('active', ready);
  }
  await set('ready', null);
  await prune([name(ready), active && name(active)].filter(Boolean));
}

self.addEventListener('install', e => e.waitUntil(first().catch(() => {}).then(() => self.skipWaiting())));
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()));

// from `cache` without waiting; a file it lacks from the server, kept (for a build's pinned pkg, `?v=`, only when the
// server still has that build: the pair never mixes)
async function serve(req, cache, key, pinned) {
  const c = await caches.open(cache);
  const hit = await c.match(key, { ignoreSearch: true });
  if (hit) return hit;
  const [r, now] = await Promise.all([network(req), pinned ? latest() : null]);
  if (r.ok && (!pinned || now === pinned)) await c.put(key, r.clone());
  return r;
}

self.addEventListener('fetch', e => {
  const req = e.request;
  const url = new URL(req.url);
  if (req.method !== 'GET' || url.origin !== location.origin || !url.href.startsWith(scope.href)) return;
  const path = url.pathname.slice(scope.pathname.length);
  e.respondWith((async () => {
    if (req.mode === 'navigate' && (path === '' || path === 'index.html')) await promote();
    const active = await get('active');
    if (!active) return fetch(req);
    // version.txt: the build this page is (served from the cache, so the page pins its pkg to it); `?next` the build
    // a Refresh would open: the ready one once downloaded, else this one (and the check for a newer one starts)
    if (path === 'version.txt') {
      if (url.searchParams.has('next')) {
        e.waitUntil(stage());
        return new Response((await get('ready')) ?? active);
      }
      return new Response(active);
    }
    // asked to come fresh (site.js's stats.json): the server first, the cache offline (kept only for a plain
    // `no-cache` file: a `no-store` or query request is the page talking to the server)
    if (req.mode !== 'navigate' && (req.cache === 'no-cache' || req.cache === 'no-store')) {
      return fetch(req).then(r => {
        if (r.ok && req.cache === 'no-cache' && !url.search) { const copy = r.clone(); e.waitUntil(caches.open(name(active)).then(c => c.put(plain(req), copy))); }
        return r;
      }, async () => (await caches.match(plain(req), { cacheName: name(active) })) ?? Response.error());
    }
    if (req.mode === 'navigate') e.waitUntil(stage());
    const v = url.searchParams.get('v');
    const pinned = v && valid(v) && (await caches.has(name(v))) ? v : null;
    const key = req.mode === 'navigate' && (path === '' || path === 'index.html') ? local('./') : plain(req);
    return serve(req, name(pinned ?? active), key, pinned).catch(() => Response.error());
  })());
});
