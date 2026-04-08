import { afterEach, describe, expect, it, vi } from 'vitest'
import { fileURLToPath, pathToFileURL } from 'url'
import { resolve } from 'path'

type EventListenerMap = {
  install?: (event: { waitUntil: (promise: Promise<unknown>) => void }) => void
  activate?: (event: { waitUntil: (promise: Promise<unknown>) => void }) => void
  fetch?: (event: {
    request: Request
    respondWith: (response: Promise<Response>) => void
    waitUntil: (promise: Promise<unknown>) => void
  }) => void
}

type MockCache = {
  match: (request: Request) => Promise<Response | undefined>
  put: (request: Request, response: Response) => Promise<void>
  delete: (request: Request) => Promise<boolean>
  keys: () => Promise<Request[]>
}

function createMockCache(initialEntries: Array<[string, Response]> = []): MockCache {
  const entries = new Map<string, Response>(
    initialEntries.map(([url, response]) => [url, response.clone()]),
  )

  return {
    async match(request) {
      const response = entries.get(request.url)
      return response?.clone()
    },
    async put(request, response) {
      entries.set(request.url, response.clone())
    },
    async delete(request) {
      return entries.delete(request.url)
    },
    async keys() {
      return [...entries.keys()].map((url) => new Request(url))
    },
  }
}

async function loadServiceWorkerWithMocks(options?: {
  initialCacheEntries?: Array<[string, Response]>
  cacheNames?: string[]
  fetchImpl?: (request: Request) => Promise<Response>
}) {
  const listeners: EventListenerMap = {}
  const cache = createMockCache(options?.initialCacheEntries)
  const deletedCacheNames: string[] = []

  const cachesMock = {
    async open(_cacheName: string) {
      return cache
    },
    async keys() {
      return options?.cacheNames ?? ['rapidbro-osm-tiles-v1']
    },
    async delete(cacheName: string) {
      deletedCacheNames.push(cacheName)
      return true
    },
  }

  const skipWaiting = vi.fn(async () => undefined)
  const claim = vi.fn(async () => undefined)
  const addEventListener = vi.fn(
    (event: keyof EventListenerMap, handler: EventListenerMap[keyof EventListenerMap]) => {
      listeners[event] = handler as never
    },
  )

  const fetchMock = vi.fn(
    options?.fetchImpl ??
      (async () =>
        new Response('tile', {
          status: 200,
          headers: { 'content-type': 'image/png' },
        })),
  )

  const originalSelf = globalThis.self
  const originalCaches = globalThis.caches
  const originalFetch = globalThis.fetch

  const selfMock = {
    addEventListener,
    skipWaiting,
    clients: {
      claim,
    },
  }

  Object.assign(globalThis, {
    self: selfMock,
    caches: cachesMock,
    fetch: fetchMock,
  })

  const testFileDir = fileURLToPath(new URL('.', import.meta.url))
  const swPath = resolve(testFileDir, '../public/sw.js')
  const swUrl = pathToFileURL(swPath).href
  await import(`${swUrl}?test=${Date.now()}-${Math.random()}`)

  return {
    listeners,
    cache,
    fetchMock,
    deletedCacheNames,
    skipWaiting,
    claim,
    restore() {
      Object.assign(globalThis, {
        self: originalSelf,
        caches: originalCaches,
        fetch: originalFetch,
      })
    },
  }
}

describe.sequential('service worker tile caching', () => {
  let envToRestore: { restore: () => void } | null = null

  afterEach(() => {
    vi.restoreAllMocks()
    if (envToRestore) {
      envToRestore.restore()
      envToRestore = null
    }
  })

  it('handles install by calling skipWaiting', async () => {
    const env = await loadServiceWorkerWithMocks()
    envToRestore = env

    const waits: Promise<unknown>[] = []
    env.listeners.install?.({
      waitUntil(promise) {
        waits.push(promise)
      },
    })

    await Promise.all(waits)
    expect(env.skipWaiting).toHaveBeenCalledOnce()
  })

  it('removes old tile caches on activate and claims clients', async () => {
    const env = await loadServiceWorkerWithMocks({
      cacheNames: ['rapidbro-osm-tiles-v0', 'rapidbro-osm-tiles-v1', 'other-cache'],
    })
    envToRestore = env

    const waits: Promise<unknown>[] = []
    env.listeners.activate?.({
      waitUntil(promise) {
        waits.push(promise)
      },
    })

    await Promise.all(waits)
    expect(env.deletedCacheNames).toEqual(['rapidbro-osm-tiles-v0'])
    expect(env.claim).toHaveBeenCalledOnce()
  })

  it('serves fresh cached OSM tile without network request', async () => {
    const tileUrl = 'https://tile.openstreetmap.org/14/12817/8050.png'
    const cachedAt = Date.now().toString()
    const env = await loadServiceWorkerWithMocks({
      initialCacheEntries: [
        [
          tileUrl,
          new Response('cached', {
            status: 200,
            headers: { 'x-rapidbro-sw-cached-at': cachedAt },
          }),
        ],
      ],
    })
    envToRestore = env

    let responsePromise: Promise<Response> | undefined
    env.listeners.fetch?.({
      request: new Request(tileUrl),
      respondWith(promise) {
        responsePromise = promise
      },
      waitUntil() {},
    })

    expect(responsePromise).toBeDefined()
    const response = await responsePromise
    if (!response) throw new Error('responsePromise resolved to undefined')
    expect(await response.text()).toBe('cached')
    expect(env.fetchMock).not.toHaveBeenCalled()
  })

  it('uses network and stores tile in cache on cache miss', async () => {
    const tileUrl = 'https://tile.openstreetmap.org/14/12817/8051.png'
    const env = await loadServiceWorkerWithMocks({
      fetchImpl: async () =>
        new Response('network-tile', {
          status: 200,
          headers: { 'content-type': 'image/png' },
        }),
    })
    envToRestore = env

    let responsePromise: Promise<Response> | undefined
    env.listeners.fetch?.({
      request: new Request(tileUrl),
      respondWith(promise) {
        responsePromise = promise
      },
      waitUntil() {},
    })

    const response = await responsePromise
    if (!response) throw new Error('responsePromise resolved to undefined')
    expect(await response.text()).toBe('network-tile')
    expect(env.fetchMock).toHaveBeenCalledOnce()

    const cached = await env.cache.match(new Request(tileUrl))
    expect(cached).toBeDefined()
    expect(cached?.headers.get('x-rapidbro-sw-cached-at')).toBeTruthy()
  })

  it('serves stale cached tile and refreshes in background', async () => {
    const tileUrl = 'https://tile.openstreetmap.org/14/12817/8049.png'
    const staleTimestamp = (Date.now() - 8 * 24 * 60 * 60 * 1000).toString()
    const env = await loadServiceWorkerWithMocks({
      initialCacheEntries: [
        [
          tileUrl,
          new Response('stale-cache', {
            status: 200,
            headers: { 'x-rapidbro-sw-cached-at': staleTimestamp },
          }),
        ],
      ],
      fetchImpl: async () =>
        new Response('fresh-network', {
          status: 200,
          headers: { 'content-type': 'image/png' },
        }),
    })
    envToRestore = env

    const backgroundJobs: Promise<unknown>[] = []
    let responsePromise: Promise<Response> | undefined

    env.listeners.fetch?.({
      request: new Request(tileUrl),
      respondWith(promise) {
        responsePromise = promise
      },
      waitUntil(promise) {
        backgroundJobs.push(promise)
      },
    })

    const immediateResponse = await responsePromise
    if (!immediateResponse) throw new Error('responsePromise resolved to undefined')
    expect(await immediateResponse.text()).toBe('stale-cache')

    await Promise.all(backgroundJobs)
    expect(env.fetchMock).toHaveBeenCalledOnce()

    const refreshed = await env.cache.match(new Request(tileUrl))
    expect(refreshed).toBeDefined()
    expect(await refreshed?.text()).toBe('fresh-network')
  })

  it('ignores non-OSM requests', async () => {
    const env = await loadServiceWorkerWithMocks()
    envToRestore = env

    let responded = false
    env.listeners.fetch?.({
      request: new Request('https://example.com/14/12817/8050.png'),
      respondWith() {
        responded = true
      },
      waitUntil() {},
    })

    expect(responded).toBe(false)
    expect(env.fetchMock).not.toHaveBeenCalled()
  })
})