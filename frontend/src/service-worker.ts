import { build, files, version } from '$service-worker';
const worker = self as unknown as any;
const FILES = `cache${version}`;
const to_cache = build.concat(files);
const staticAssets = new Set(to_cache);
// listen for the install events
worker.addEventListener('install', (event: any) => {
  event.waitUntil(
    caches
      .open(FILES)
      .then((cache) => cache.addAll(to_cache))
      .then(() => {
        worker.skipWaiting();
      })
  );
});
// listen for the activate events
worker.addEventListener('activate', (event: any) => {
  event.waitUntil(
    caches.keys().then(async (keys) => {
      // delete old caches
      for (const key of keys) {
        if (key !== FILES) await caches.delete(key);
      }
      worker.clients.claim();
    })
  );
});
// attempt to process HTTP requests and rely on the cache if offline
async function fetchAndCache(request: Request) {
  const cache = await caches.open(`offline${version}`);
  try {
    const response = await fetch(request);
    cache.put(request, response.clone());
    return response;
  } catch (err) {
    const response = await cache.match(request);
    if (response) return response;
    throw err;
  }
}
// listen for the fetch events
worker.addEventListener('fetch', (event: any) => {
  if (event.request.method !== 'GET' || event.request.headers.has('range')) return;
  const url = new URL(event.request.url);
  // only cache files that are local to your application
  const isHttp = url.protocol.startsWith('http');
  const isDevServerRequest =
    url.hostname === self.location.hostname && url.port !== self.location.port;
  const isStaticAsset = url.host === self.location.host && staticAssets.has(url.pathname);
  const skipBecauseUncached = event.request.cache === 'only-if-cached' && !isStaticAsset;
  if (isHttp && !isDevServerRequest && !skipBecauseUncached) {
    event.respondWith(
      (async () => {
        // always serve static files and bundler-generated assets from cache.
        // if your application has other URLs with data that will never change,
        // set this variable to true for them and they will only be fetched once.
        const cachedAsset = isStaticAsset && (await caches.match(event.request));
        return cachedAsset || fetchAndCache(event.request);
      })()
    );
  }
});

// Listen for push events
worker.addEventListener('push', (event: any) => {
  // console.log('received push event');
  const data = event.data ? event.data.json() : {};
  const title = data.title || 'Ratings';
  const options = {
    body: data.body || 'Default body content',
    icon: data.icon || '/apple-icon-144x144.png',
    badge: data.badge || '/apple-icon-144x144.png',
    data: {
      url: data.url || '/',
      ...data
    }
  };

  // console.log('body: ' + options.body);
  event.waitUntil(worker.registration.showNotification(title, options));
});

// Listen for notificationclick events
worker.addEventListener('notificationclick', (event: any) => {
  event.notification.close();

  event.waitUntil(
    worker.clients
      .matchAll({ type: 'window', includeUncontrolled: true })
      .then((clientList: any) => {
        const client = clientList.find(
          (c: any) => c.url === event.notification.data.url && 'focus' in c
        );
        if (client) {
          return client.focus();
        } else if (worker.clients.openWindow) {
          return worker.clients.openWindow(event.notification.data.url);
        }
      })
  );
});
