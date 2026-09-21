// VERSION and ASSETS are injected from the completed Trunk output.
const CACHE = 'urma-wire-shell-' + VERSION;
const MARKER = '/__urma_complete__';
const urls = new Set(ASSETS.map(asset => asset.url));
async function complete(cache) { return !!(await cache.match(MARKER)); }
self.addEventListener('install', event => event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    if (await complete(cache)) return;
    try {
        for (const asset of ASSETS) {
            const response = await fetch(asset.url, {cache: 'no-store'});
            if (!response.ok || response.type === 'opaque') throw Error('Incomplete application download');
            const bytes = await response.clone().arrayBuffer();
            const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)))
                .map(n => n.toString(16).padStart(2, '0')).join('');
            if (digest !== asset.sha256) throw Error('Application changed during download');
            await cache.put(asset.url, response);
        }
        await cache.put(MARKER, new Response(VERSION));
    } catch (error) {
        await caches.delete(CACHE);
        throw error; // The existing active worker and its complete cache survive.
    }
})()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
// No skipWaiting: updates activate after previous clients close, preventing mixed assets.
// Previous complete caches deliberately survive updates and failed installations.
self.addEventListener('fetch', event => {
    const url = new URL(event.request.url);
    if (event.request.method !== 'GET' || url.origin !== self.location.origin) return;
    if (event.request.mode !== 'navigate' && !urls.has(url.pathname)) return;
    event.respondWith((async () => {
        const cache = await caches.open(CACHE);
        if (!(await complete(cache))) return fetch(event.request);
        return (await cache.match(event.request.mode === 'navigate' ? '/index.html' : url.pathname))
            || fetch(event.request);
    })());
});
self.addEventListener('message', event => {
    if (event.data !== 'offline-status' || !event.ports[0]) return;
    event.waitUntil((async () => event.ports[0].postMessage({complete: await complete(await caches.open(CACHE)), version: VERSION}))());
});
