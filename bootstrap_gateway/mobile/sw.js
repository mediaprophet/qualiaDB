const CACHE_NAME = 'qualia-edge-v1';
const ASSETS = [
    './',
    './index.html',
    './manifest.json',
    './qualia-mobile-harness.js',
    './qualia-mobile-harness_bg.wasm',
    './sync_io_worker.js'
];

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME).then(cache => cache.addAll(ASSETS))
    );
});

self.addEventListener('fetch', event => {
    event.respondWith(
        caches.match(event.request).then(response => {
            return response || fetch(event.request);
        })
    );
});
