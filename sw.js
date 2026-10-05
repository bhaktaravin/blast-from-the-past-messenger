// Service worker for the installable web app.
//
// The app needs the chat server to do anything, so this is about fast, reliable
// startup rather than offline use:
//  - Pages are network-first, so a new deploy is picked up on the next launch,
//    with the cached copy as a fallback on a flaky connection.
//  - Trunk's build output (chatmessagediscordclone-<hash>.js / _bg.wasm) is
//    content-hashed and never changes, so it is cache-first. When a new hash
//    is cached, older builds are dropped so the multi-MB wasm doesn't pile up.
//  - Only same-origin GETs are touched; the WebSocket and API traffic to the
//    chat server bypass the worker entirely.

const CACHE = 'bftp-v1';
// The app may live under a subpath (GitHub Pages), so work relative to our scope
const BASE = new URL(self.registration.scope).pathname;
const BUILD_PREFIX = BASE + 'chatmessagediscordclone-';

self.addEventListener('install', () => self.skipWaiting());

self.addEventListener('activate', (event) => {
    event.waitUntil((async () => {
        for (const name of await caches.keys()) {
            if (name !== CACHE) await caches.delete(name);
        }
        await self.clients.claim();
    })());
});

self.addEventListener('fetch', (event) => {
    const req = event.request;
    if (req.method !== 'GET') return;
    const url = new URL(req.url);
    if (url.origin !== self.location.origin) return;

    if (req.mode === 'navigate') {
        event.respondWith(networkFirst(req));
    } else if (url.pathname.startsWith(BUILD_PREFIX)) {
        event.respondWith(cacheFirstBuild(req, url));
    } else {
        event.respondWith(staleWhileRevalidate(event, req));
    }
});

async function networkFirst(req) {
    const cache = await caches.open(CACHE);
    try {
        const res = await fetch(req);
        if (res.ok) await cache.put(BASE, res.clone());
        return res;
    } catch (err) {
        const cached = await cache.match(BASE);
        if (cached) return cached;
        throw err;
    }
}

async function cacheFirstBuild(req, url) {
    const cache = await caches.open(CACHE);
    const cached = await cache.match(req);
    if (cached) return cached;
    const res = await fetch(req);
    if (res.ok) {
        // Drop other builds of the same file type (.js / _bg.wasm)
        const suffix = url.pathname.endsWith('_bg.wasm') ? '_bg.wasm' : url.pathname.slice(url.pathname.lastIndexOf('.'));
        for (const old of await cache.keys()) {
            const path = new URL(old.url).pathname;
            if (path !== url.pathname && path.startsWith(BUILD_PREFIX) && path.endsWith(suffix)) {
                await cache.delete(old);
            }
        }
        await cache.put(req, res.clone());
    }
    return res;
}

async function staleWhileRevalidate(event, req) {
    const cache = await caches.open(CACHE);
    const cached = await cache.match(req);
    const fresh = fetch(req).then((res) => {
        if (res.ok) cache.put(req, res.clone());
        return res;
    });
    if (cached) {
        event.waitUntil(fresh.catch(() => {}));
        return cached;
    }
    return fresh;
}
