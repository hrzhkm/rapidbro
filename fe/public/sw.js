const TILE_CACHE_PREFIX = 'rapidbro-osm-tiles-'
const TILE_CACHE_VERSION = 'v1'
const TILE_CACHE_NAME = `${TILE_CACHE_PREFIX}${TILE_CACHE_VERSION}`
const TILE_CACHE_MAX_ENTRIES = 240
const TILE_CACHE_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000
const TILE_CACHE_TIMESTAMP_HEADER = 'x-rapidbro-sw-cached-at'

function isOpenStreetMapTileRequest(request) {
  if (request.method !== 'GET') {
    return false
  }

  const url = new URL(request.url)
  if (url.origin !== 'https://tile.openstreetmap.org') {
    return false
  }

  const pathMatch = url.pathname.match(/^\/(\d+)\/(\d+)\/(\d+)\.png$/)
  return pathMatch !== null
}

function getCachedAgeMs(cachedResponse) {
  const cachedAtRaw = cachedResponse.headers.get(TILE_CACHE_TIMESTAMP_HEADER)
  if (!cachedAtRaw) {
    return Number.POSITIVE_INFINITY
  }

  const cachedAt = Number.parseInt(cachedAtRaw, 10)
  if (!Number.isFinite(cachedAt)) {
    return Number.POSITIVE_INFINITY
  }

  return Date.now() - cachedAt
}

async function addCachedAtHeader(response) {
  if (!response.ok || response.type === 'opaque') {
    return response
  }

  const buffer = await response.arrayBuffer()
  const headers = new Headers(response.headers)
  headers.set(TILE_CACHE_TIMESTAMP_HEADER, Date.now().toString())

  return new Response(buffer, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}

async function pruneCache(cache) {
  const requests = await cache.keys()
  if (requests.length <= TILE_CACHE_MAX_ENTRIES) {
    return
  }

  const excessCount = requests.length - TILE_CACHE_MAX_ENTRIES
  for (let index = 0; index < excessCount; index += 1) {
    await cache.delete(requests[index])
  }
}

async function writeTileToCache(request, response) {
  if (!response.ok) {
    return
  }

  const cache = await caches.open(TILE_CACHE_NAME)
  const responseToCache = await addCachedAtHeader(response.clone())
  await cache.put(request, responseToCache)
  await pruneCache(cache)
}

async function fetchAndCacheTile(request) {
  const networkResponse = await fetch(request)
  await writeTileToCache(request, networkResponse)
  return networkResponse
}

self.addEventListener('install', (event) => {
  event.waitUntil(self.skipWaiting())
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const cacheNames = await caches.keys()
      await Promise.all(
        cacheNames.map((cacheName) => {
          if (
            cacheName.startsWith(TILE_CACHE_PREFIX) &&
            cacheName !== TILE_CACHE_NAME
          ) {
            return caches.delete(cacheName)
          }
          return Promise.resolve()
        }),
      )
      await self.clients.claim()
    })(),
  )
})

self.addEventListener('fetch', (event) => {
  if (!isOpenStreetMapTileRequest(event.request)) {
    return
  }

  event.respondWith(
    (async () => {
      const cache = await caches.open(TILE_CACHE_NAME)
      const cachedResponse = await cache.match(event.request)

      if (cachedResponse) {
        const cachedAgeMs = getCachedAgeMs(cachedResponse)
        if (cachedAgeMs <= TILE_CACHE_MAX_AGE_MS) {
          return cachedResponse
        }

        event.waitUntil(
          fetchAndCacheTile(event.request).catch(() => {
            return undefined
          }),
        )
        return cachedResponse
      }

      return fetchAndCacheTile(event.request)
    })(),
  )
})
