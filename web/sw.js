// Offline play: the page, its scripts and the game are kept in the browser's cache and served from it, refreshed from
// the network whenever it answers (the disc pack is in OPFS, never in this cache: nothing from the disc is hosted).
const CACHE = 'hst';
const SHELL = ['./', 'touch.js', 'worker.js', 'manifest.webmanifest', 'icon.svg', 'icon-192.png', 'pkg/hst.js', 'pkg/hst_bg.wasm'];

self.addEventListener('install', e => e.waitUntil(caches.open(CACHE).then(c => c.addAll(SHELL)).then(() => self.skipWaiting())));
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()));

// network first (a new build shows up on the next visit), the cache when offline
self.addEventListener('fetch', e => {
  if (e.request.method !== 'GET' || new URL(e.request.url).origin !== location.origin) return;
  e.respondWith(
    fetch(e.request)
      .then(r => {
        if (r.ok) {
          const copy = r.clone();
          caches.open(CACHE).then(c => c.put(e.request, copy));
        }
        return r;
      })
      .catch(() => caches.match(e.request, { ignoreSearch: true })),
  );
});
