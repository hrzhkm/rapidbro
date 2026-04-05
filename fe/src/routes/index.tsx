import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, LocateFixed, MapPin } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { BusRouteMap } from '@/components/BusRouteMap'
import { BusRouteLine } from '@/components/BusRouteLine'
import { MapPanelShell } from '@/components/MapPanelShell'
import { Button } from '@/components/ui/button'
import { mergeBusesWithGraceHold } from '@/lib/bus-grace'
import { getGeolocationPosition } from '@/lib/geolocation'
import {
  buildRoutePrefetchWarning,
  buildVisibleRouteLayers,
  shouldShowRouteStopMarkers,
} from '@/lib/route-map-layers'
import { type RouteShape } from '@/lib/route-geometry'

export const Route = createFileRoute('/')({ component: App })

type NearestStopResponse = {
  stop_id: string
  stop_name: string
  stop_desc: string
  stop_lat: number
  stop_lon: number
  distance_km: number
  distance_meters: number
}

type BusEta = {
  route_id: string
  bus_no: string
  current_lat: number
  current_lon: number
  current_stop_id: string
  current_stop_name: string
  current_sequence: number
  stop_resolution_source: 'live' | 'derived'
  stops_away: number
  distance_km: number
  speed_kmh: number
  eta_minutes: number
}

type StopRouteSummary = {
  route_id: string
  route_short_name: string
  route_long_name: string
}

type StopRoutesResponse = {
  stop_id: string
  routes: StopRouteSummary[]
}

type RouteStop = {
  stop_id: string
  stop_name: string
  stop_desc: string
  stop_lat: number
  stop_lon: number
  sequence: number
}

type RouteStopsResponse = {
  route_id: string
  route_short_name: string
  route_long_name: string
  stops: RouteStop[]
}

type UserCoords = {
  lat: number
  lon: number
}

const panelSectionClass =
  'rounded-2xl border border-amber-200/80 bg-white/78 p-3 text-slate-800 shadow-[inset_0_1px_0_rgba(255,255,255,0.55)]'
const panelSubtleTextClass = 'text-xs text-slate-600'
const BUS_GRACE_WINDOW_MS = 5 * 60 * 1000

// Perlis / northern Kedah bounding box — routes to /kangar/* endpoints.
function isKangarRegion(lat: number, lon: number): boolean {
  return lat >= 5.5 && lat <= 6.7 && lon >= 99.5 && lon <= 101.0
}

function App() {
  const apiBaseUrl = useMemo(
    () => import.meta.env.VITE_BE_URL ?? 'http://localhost:3030',
    [],
  )
  const [apiRegionPrefix, setApiRegionPrefix] = useState<'' | '/kangar'>('')
  const hasAutoRequestedNearestStopRef = useRef(false)
  const nearestStopEtaRef = useRef<BusEta[]>([])
  const lastNonEmptyEtaAtMsRef = useRef<number | null>(null)
  const etaGraceStopIdRef = useRef<string | null>(null)
  const [coords, setCoords] = useState<UserCoords | null>(null)
  const [nearestStop, setNearestStop] = useState<NearestStopResponse | null>(
    null,
  )
  const [nearestStopEta, setNearestStopEta] = useState<BusEta[]>([])
  const [stopRoutes, setStopRoutes] = useState<StopRouteSummary[]>([])
  const [routeStopsByRoute, setRouteStopsByRoute] = useState<
    Record<string, RouteStopsResponse>
  >({})
  const [routeShapesByKey, setRouteShapesByKey] = useState<
    Record<string, RouteShape>
  >({})
  const [loadingRouteShapesByKey, setLoadingRouteShapesByKey] = useState<
    Record<string, boolean>
  >({})
  const [selectedMapRouteId, setSelectedMapRouteId] = useState<string | null>(
    null,
  )
  const [selectedBusKey, setSelectedBusKey] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [isLoadingEta, setIsLoadingEta] = useState(false)
  const [isLoadingRoutes, setIsLoadingRoutes] = useState(false)
  const [isLoadingSelectedRoute, setIsLoadingSelectedRoute] = useState(false)
  const [lastFetchedAt, setLastFetchedAt] = useState<Date | null>(null)
  const [lastNonEmptyEtaAt, setLastNonEmptyEtaAt] = useState<Date | null>(null)
  const [isEtaGraceHeld, setIsEtaGraceHeld] = useState(false)
  const [locationPermission, setLocationPermission] = useState<PermissionState | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [etaErrorMessage, setEtaErrorMessage] = useState<string | null>(null)
  const [routeErrorMessage, setRouteErrorMessage] = useState<string | null>(
    null,
  )
  const [selectedRouteErrorMessage, setSelectedRouteErrorMessage] = useState<
    string | null
  >(null)

  const getBusKey = (eta: BusEta) =>
    `${eta.route_id}-${eta.bus_no}-${eta.current_stop_id}`

  const selectedBus =
    selectedBusKey === null
      ? null
      : nearestStopEta.find((eta) => getBusKey(eta) === selectedBusKey) ?? null
  const selectedRouteStops = selectedBus
    ? routeStopsByRoute[selectedBus.route_id] ?? null
    : null
  const getRouteShapeCacheKey = (routeId: string, stopId?: string | null) =>
    stopId ? `${routeId}::${stopId}` : routeId
  const getRouteShape = (routeId: string, stopId?: string | null) =>
    routeShapesByKey[getRouteShapeCacheKey(routeId, stopId)] ??
    routeShapesByKey[routeId] ??
    null
  const selectedRouteShape = selectedBus
    ? getRouteShape(selectedBus.route_id, nearestStop?.stop_id)
    : null
  const selectedRouteShapeCacheKey = selectedBus
    ? getRouteShapeCacheKey(selectedBus.route_id, nearestStop?.stop_id)
    : null
  const isLoadingSelectedRouteShape = selectedRouteShapeCacheKey
    ? loadingRouteShapesByKey[selectedRouteShapeCacheKey] === true
    : false
  const mapRouteIds = useMemo(
    () => stopRoutes.map((route) => route.route_id),
    [stopRoutes],
  )
  const visibleRouteLayers = useMemo(
    () =>
      buildVisibleRouteLayers({
        routeIds: mapRouteIds,
        selectedRouteId: selectedMapRouteId,
        routeStopsByRoute,
        resolveShape: (routeId) => getRouteShape(routeId, nearestStop?.stop_id),
      }),
    [mapRouteIds, selectedMapRouteId, routeStopsByRoute, routeShapesByKey, nearestStop],
  )
  const visibleMapBuses = useMemo(
    () =>
      selectedMapRouteId
        ? nearestStopEta.filter((eta) => eta.route_id === selectedMapRouteId)
        : nearestStopEta,
    [nearestStopEta, selectedMapRouteId],
  )

  useEffect(() => {
    nearestStopEtaRef.current = nearestStopEta
  }, [nearestStopEta])

  const fetchNearestStop = async (lat: number, lon: number, prefix = apiRegionPrefix) => {
    const params = new URLSearchParams({
      lat: lat.toString(),
      lon: lon.toString(),
    })
    const response = await fetch(
      `${apiBaseUrl}${prefix}/stops/nearest?${params.toString()}`,
    )
    if (!response.ok) {
      const fallbackMessage = 'Unable to fetch nearest bus stop'
      const body = (await response.json().catch(() => null)) as {
        error?: string
      } | null
      throw new Error(body?.error ?? fallbackMessage)
    }

    const data = (await response.json()) as NearestStopResponse
    setNearestStop(data)
    return data
  }

  const fetchEtaToStop = async (stopId: string, prefix = apiRegionPrefix) => {
    setEtaErrorMessage(null)
    setSelectedRouteErrorMessage(null)
    setIsLoadingEta(true)
    const isSameStopAsGraceState = etaGraceStopIdRef.current === stopId
    const previousEtaForStop = isSameStopAsGraceState ? nearestStopEtaRef.current : []
    const lastNonEmptyEtaAtMsForStop = isSameStopAsGraceState
      ? lastNonEmptyEtaAtMsRef.current
      : null

    try {
      const response = await fetch(
        `${apiBaseUrl}${prefix}/stops/${encodeURIComponent(stopId)}/eta`,
      )
      if (!response.ok) {
        const fallbackMessage = 'Unable to fetch ETA for nearest stop'
        const body = (await response.json().catch(() => null)) as {
          error?: string
        } | null
        setEtaErrorMessage(body?.error ?? fallbackMessage)
        const merged = mergeBusesWithGraceHold({
          incoming: [],
          previous: previousEtaForStop,
          fetchSucceeded: false,
          nowMs: Date.now(),
          graceWindowMs: BUS_GRACE_WINDOW_MS,
          lastNonEmptyAtMs: lastNonEmptyEtaAtMsForStop,
        })
        setNearestStopEta(merged.buses)
        setIsEtaGraceHeld(merged.isGraceHeld)
        setLastNonEmptyEtaAt(
          merged.lastNonEmptyAtMs === null ? null : new Date(merged.lastNonEmptyAtMs),
        )
        lastNonEmptyEtaAtMsRef.current = merged.lastNonEmptyAtMs
        etaGraceStopIdRef.current = stopId
        return
      }

      const data = (await response.json()) as BusEta[]
      const merged = mergeBusesWithGraceHold({
        incoming: data,
        previous: previousEtaForStop,
        fetchSucceeded: true,
        nowMs: Date.now(),
        graceWindowMs: BUS_GRACE_WINDOW_MS,
        lastNonEmptyAtMs: lastNonEmptyEtaAtMsForStop,
      })
      setNearestStopEta(merged.buses)
      setIsEtaGraceHeld(merged.isGraceHeld)
      setLastNonEmptyEtaAt(
        merged.lastNonEmptyAtMs === null ? null : new Date(merged.lastNonEmptyAtMs),
      )
      lastNonEmptyEtaAtMsRef.current = merged.lastNonEmptyAtMs
      etaGraceStopIdRef.current = stopId
    } catch (error) {
      setEtaErrorMessage(
        error instanceof Error
          ? error.message
          : 'Unable to fetch ETA for nearest stop',
      )
      const merged = mergeBusesWithGraceHold({
        incoming: [],
        previous: previousEtaForStop,
        fetchSucceeded: false,
        nowMs: Date.now(),
        graceWindowMs: BUS_GRACE_WINDOW_MS,
        lastNonEmptyAtMs: lastNonEmptyEtaAtMsForStop,
      })
      setNearestStopEta(merged.buses)
      setIsEtaGraceHeld(merged.isGraceHeld)
      setLastNonEmptyEtaAt(
        merged.lastNonEmptyAtMs === null ? null : new Date(merged.lastNonEmptyAtMs),
      )
      lastNonEmptyEtaAtMsRef.current = merged.lastNonEmptyAtMs
      etaGraceStopIdRef.current = stopId
    } finally {
      setIsLoadingEta(false)
    }
  }

  const fetchRouteStops = async (
    routeId: string,
    options: {
      setSelectedErrorState?: boolean
      setSelectedLoadingState?: boolean
    } = {},
  ) => {
    const {
      setSelectedErrorState = true,
      setSelectedLoadingState = true,
    } = options

    if (routeStopsByRoute[routeId]) {
      return routeStopsByRoute[routeId]
    }

    if (setSelectedErrorState) {
      setSelectedRouteErrorMessage(null)
    }
    if (setSelectedLoadingState) {
      setIsLoadingSelectedRoute(true)
    }

    try {
      const response = await fetch(
        `${apiBaseUrl}${apiRegionPrefix}/route/${encodeURIComponent(routeId)}/stops`,
      )
      if (!response.ok) {
        const fallbackMessage = 'Unable to fetch route stops'
        const body = (await response.json().catch(() => null)) as {
          error?: string
        } | null
        throw new Error(body?.error ?? fallbackMessage)
      }

      const data = (await response.json()) as RouteStopsResponse
      setRouteStopsByRoute((current) => ({
        ...current,
        [routeId]: data,
      }))
      return data
    } catch (error) {
      const message =
        error instanceof Error ? error.message : 'Unable to fetch route stops'
      if (setSelectedErrorState) {
        setSelectedRouteErrorMessage(message)
      }
      throw error
    } finally {
      if (setSelectedLoadingState) {
        setIsLoadingSelectedRoute(false)
      }
    }
  }

  const fetchRouteShape = async (
    routeId: string,
    stopId?: string | null,
    options: {
      setSelectedErrorState?: boolean
    } = {},
  ) => {
    const { setSelectedErrorState = true } = options
    const cacheKey = getRouteShapeCacheKey(routeId, stopId)
    if (routeShapesByKey[cacheKey]) {
      return routeShapesByKey[cacheKey]
    }

    setLoadingRouteShapesByKey((current) => ({
      ...current,
      [cacheKey]: true,
    }))
    if (setSelectedErrorState) {
      setSelectedRouteErrorMessage(null)
    }

    try {
      const params = stopId ? `?stop_id=${encodeURIComponent(stopId)}` : ''
      const response = await fetch(
        `${apiBaseUrl}${apiRegionPrefix}/route/${encodeURIComponent(routeId)}/shape${params}`,
      )
      if (!response.ok) {
        const fallbackMessage = 'Unable to fetch route shape'
        const body = (await response.json().catch(() => null)) as {
          error?: string
        } | null
        throw new Error(body?.error ?? fallbackMessage)
      }

      const data = (await response.json()) as RouteShape
      setRouteShapesByKey((current) => ({
        ...current,
        [cacheKey]: data,
      }))
      return data
    } catch (error) {
      if (setSelectedErrorState) {
        setSelectedRouteErrorMessage(
          error instanceof Error ? error.message : 'Unable to fetch route shape',
        )
      }
      throw error
    } finally {
      setLoadingRouteShapesByKey((current) => ({
        ...current,
        [cacheKey]: false,
      }))
    }
  }

  const fetchRoutesForStop = async (stopId: string, prefix = apiRegionPrefix) => {
    setRouteErrorMessage(null)
    setStopRoutes([])
    setIsLoadingRoutes(true)

    try {
      const response = await fetch(
        `${apiBaseUrl}${prefix}/stops/${encodeURIComponent(stopId)}/routes`,
      )
      if (!response.ok) {
        const fallbackMessage = 'Unable to fetch routes for nearest stop'
        const body = (await response.json().catch(() => null)) as {
          error?: string
        } | null
        throw new Error(body?.error ?? fallbackMessage)
      }

      const data = (await response.json()) as StopRoutesResponse
      setStopRoutes(data.routes)
      setSelectedMapRouteId(null)

      if (data.routes.length > 0) {
        const prefetchResults = await Promise.allSettled(
          data.routes.map(async (route) => {
            await Promise.all([
              fetchRouteStops(route.route_id, {
                setSelectedErrorState: false,
                setSelectedLoadingState: false,
              }),
              fetchRouteShape(
                route.route_id,
                stopId,
                {
                  setSelectedErrorState: false,
                },
              ),
            ])
            return route.route_id
          }),
        )

        const failedRouteIds = prefetchResults.flatMap((result, index) =>
          result.status === 'rejected'
            ? [data.routes[index]?.route_id ?? 'Unknown route']
            : [],
        )
        const prefetchWarning = buildRoutePrefetchWarning({
          failedRouteIds,
          totalRouteCount: data.routes.length,
        })
        setRouteErrorMessage(prefetchWarning)
      }
    } catch (error) {
      setRouteErrorMessage(
        error instanceof Error
          ? error.message
          : 'Unable to fetch routes for nearest stop',
      )
    } finally {
      setIsLoadingRoutes(false)
    }
  }

  const handleSelectBus = async (eta: BusEta) => {
    setSelectedBusKey(getBusKey(eta))
    setSelectedMapRouteId(eta.route_id)
    setSelectedRouteErrorMessage(null)

    if (
      routeStopsByRoute[eta.route_id] &&
      getRouteShape(eta.route_id, nearestStop?.stop_id)
    ) {
      return
    }

    try {
      await Promise.all([
        fetchRouteStops(eta.route_id),
        fetchRouteShape(eta.route_id, nearestStop?.stop_id),
      ])
    } catch {
      // Error state is handled above so selected details can still render.
    }
  }

  const handleSelectMapRoute = async (routeId: string) => {
    setSelectedMapRouteId(routeId)
    setSelectedRouteErrorMessage(null)

    if (routeStopsByRoute[routeId] && getRouteShape(routeId, nearestStop?.stop_id)) {
      return
    }

    try {
      await Promise.all([
        fetchRouteStops(routeId),
        fetchRouteShape(routeId, nearestStop?.stop_id),
      ])
    } catch {
      // Map route error is shown via selectedRouteErrorMessage.
    }
  }

  const handleSelectAllRoutes = async () => {
    setSelectedMapRouteId(null)
    setSelectedRouteErrorMessage(null)
    setRouteErrorMessage(null)

    if (!nearestStop || stopRoutes.length === 0) {
      return
    }

    const uncachedRouteIds = stopRoutes
      .map((route) => route.route_id)
      .filter(
        (routeId) =>
          !routeStopsByRoute[routeId] ||
          getRouteShape(routeId, nearestStop.stop_id) === null,
      )
    if (uncachedRouteIds.length === 0) {
      return
    }

    const prefetchResults = await Promise.allSettled(
      uncachedRouteIds.map(async (routeId) => {
        await Promise.all([
          fetchRouteStops(routeId, {
            setSelectedErrorState: false,
            setSelectedLoadingState: false,
          }),
          fetchRouteShape(routeId, nearestStop.stop_id, {
            setSelectedErrorState: false,
          }),
        ])
        return routeId
      }),
    )

    const failedRouteIds = prefetchResults.flatMap((result, index) =>
      result.status === 'rejected'
        ? [uncachedRouteIds[index] ?? 'Unknown route']
        : [],
    )
    const prefetchWarning = buildRoutePrefetchWarning({
      failedRouteIds,
      totalRouteCount: uncachedRouteIds.length,
    })
    setRouteErrorMessage(prefetchWarning)
  }

  const handleFindNearestStop = async ({
    preserveCurrentData = false,
  }: {
    preserveCurrentData?: boolean
  } = {}) => {
    setErrorMessage(null)
    setEtaErrorMessage(null)
    setRouteErrorMessage(null)
    setSelectedRouteErrorMessage(null)

    if (!preserveCurrentData) {
      setIsLoading(true)
    }

    if (!('geolocation' in navigator)) {
      if (!preserveCurrentData) {
        setIsLoading(false)
      }
      setErrorMessage('Geolocation is not supported by this browser.')
      return
    }

    try {
      const position = await getGeolocationPosition(navigator.geolocation)
      const lat = position.coords.latitude
      const lon = position.coords.longitude
      setCoords({ lat, lon })

      const prefix = isKangarRegion(lat, lon) ? '/kangar' : ''
      setApiRegionPrefix(prefix)

      const nearestStopData = await fetchNearestStop(lat, lon, prefix)
      await Promise.all([
        fetchEtaToStop(nearestStopData.stop_id, prefix),
        fetchRoutesForStop(nearestStopData.stop_id, prefix),
      ])
      setLastFetchedAt(new Date())
    } catch (error) {
      setErrorMessage(
        error instanceof Error
          ? error.message
          : 'Unable to read your location.',
      )
    } finally {
      if (!preserveCurrentData) {
        setIsLoading(false)
      }
    }
  }

  useEffect(() => {
    if (hasAutoRequestedNearestStopRef.current) return

    const checkPermissionAndAutoRequest = async () => {
      let state: PermissionState = 'prompt'

      if ('permissions' in navigator) {
        try {
          const result = await navigator.permissions.query({ name: 'geolocation' })
          state = result.state
          setLocationPermission(result.state)
          result.onchange = () => setLocationPermission(result.state)
        } catch {
          setLocationPermission('prompt')
        }
      } else {
        setLocationPermission('prompt')
      }

      if (state === 'granted') {
        hasAutoRequestedNearestStopRef.current = true
        void handleFindNearestStop()
      }
    }

    void checkPermissionAndAutoRequest()
  }, [])

  useEffect(() => {
    const id = window.setInterval(() => {
      void handleFindNearestStop({ preserveCurrentData: true })
    }, 30000)

    return () => window.clearInterval(id)
  }, [])

  const mapContent =
    visibleRouteLayers.length > 0 ? (
      <BusRouteMap
        className="h-full"
        fullScreen
        showLegend={false}
        routeLayers={visibleRouteLayers}
        stops={[]}
        trackedStop={
          nearestStop
            ? {
                stop_id: nearestStop.stop_id,
                stop_name: nearestStop.stop_name,
                stop_lat: nearestStop.stop_lat,
                stop_lon: nearestStop.stop_lon,
              }
            : null
        }
        showStopMarkers={shouldShowRouteStopMarkers(
          selectedMapRouteId,
          visibleRouteLayers.length,
        )}
        buses={visibleMapBuses.map((eta) => ({
          id: getBusKey(eta),
          label: `Bus ${eta.bus_no}`,
          lat: eta.current_lat,
          lon: eta.current_lon,
          isSelected:
            selectedBusKey !== null && selectedBusKey === getBusKey(eta),
        }))}
        currentStopId={
          selectedBus?.route_id === selectedMapRouteId
            ? selectedBus.current_stop_id
            : null
        }
        targetStopId={nearestStop?.stop_id ?? null}
      />
    ) : (
      <div className="flex h-full items-center justify-center bg-[radial-gradient(circle_at_20%_12%,rgba(252,211,77,0.45),transparent_44%),radial-gradient(circle_at_80%_85%,rgba(34,211,238,0.24),transparent_36%),linear-gradient(120deg,#fff7ed_0%,#fffbeb_52%,#fef3c7_100%)] px-6 text-center text-slate-800">
        <div>
          <p className="font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-lg font-semibold tracking-tight text-amber-900">
            Live Route Map
          </p>
          <p className="mt-2 text-sm text-slate-700">
            {isLoadingRoutes
              ? 'Loading the best route view for your nearest stop...'
              : 'Pick a route from the panel to render its live map.'}
          </p>
          {selectedRouteErrorMessage ? (
            <p className="mt-2 text-xs text-rose-700">{selectedRouteErrorMessage}</p>
          ) : null}
        </div>
      </div>
    )

  const locationOverlay =
    locationPermission !== 'granted' && !coords ? (
      <div className="w-full max-w-sm rounded-2xl border border-amber-300/80 bg-white/90 p-5 shadow-2xl backdrop-blur-md">
        <div className="flex items-start gap-4">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-amber-100">
            <LocateFixed className="h-5 w-5 text-amber-600" />
          </div>
          <div className="flex-1">
            {locationPermission === 'denied' ? (
              <>
                <p className="text-sm font-semibold text-rose-800">Location access blocked</p>
                <p className="mt-1 text-xs text-slate-600">
                  Enable location access for this site in your browser settings, then refresh the page.
                </p>
              </>
            ) : (
              <>
                <p className="text-sm font-semibold text-amber-900">Allow location access</p>
                <p className="mt-1 text-xs text-slate-600">
                  RapidBro needs your location to find nearby bus stops and show real-time bus arrivals.
                </p>
                <Button
                  type="button"
                  onClick={() => {
                    hasAutoRequestedNearestStopRef.current = true
                    void handleFindNearestStop()
                  }}
                  disabled={isLoading}
                  size="sm"
                  className="mt-4 w-full bg-amber-500 text-slate-900 hover:bg-amber-400"
                >
                  {isLoading ? (
                    <>
                      <LoaderCircle className="animate-spin" />
                      Getting location...
                    </>
                  ) : (
                    <>
                      <LocateFixed />
                      Share my location
                    </>
                  )}
                </Button>
              </>
            )}
          </div>
        </div>
      </div>
    ) : null

  return (
    <MapPanelShell
      map={mapContent}
      mapOverlay={locationOverlay}
      panelTitle="Nearest Bus Stop"
      panelDescription="Track buses around your current stop with live route context."
      panelStatus={
        <div>
          <p className="text-slate-600">
            {lastFetchedAt
              ? `Updated ${lastFetchedAt.toLocaleTimeString()}`
              : 'Waiting for location update'}
          </p>
          {isEtaGraceHeld ? (
            <p className="text-xs text-amber-700">
              Showing last live buses from{' '}
              {lastNonEmptyEtaAt
                ? lastNonEmptyEtaAt.toLocaleTimeString()
                : 'recent update'}{' '}
              (temporary stale data)
            </p>
          ) : null}
        </div>
      }
      panelActions={
        <div className="space-y-2">
          <Button
            type="button"
            onClick={() => handleFindNearestStop()}
            disabled={isLoading}
            className="w-full bg-amber-500 text-slate-900 hover:bg-amber-400"
          >
            {isLoading ? (
              <>
                <LoaderCircle className="animate-spin" />
                Finding nearest stop...
              </>
            ) : (
              <>
                <LocateFixed />
                {coords ? 'Refresh nearest stop' : 'Find nearest stop'}
              </>
            )}
          </Button>
          {coords ? (
            <p className={panelSubtleTextClass}>
              Lat {coords.lat.toFixed(5)} · Lon {coords.lon.toFixed(5)}
            </p>
          ) : null}
        </div>
      }
    >
      {locationPermission !== 'granted' && !coords ? (
        <section className="rounded-2xl border border-amber-300/80 bg-amber-50/90 p-4 text-slate-800 shadow-[inset_0_1px_0_rgba(255,255,255,0.55)]">
          <div className="flex items-start gap-3">
            <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-amber-200">
              <LocateFixed className="h-4 w-4 text-amber-700" />
            </div>
            <div className="flex-1">
              {locationPermission === 'denied' ? (
                <>
                  <p className="text-sm font-semibold text-rose-800">Location access blocked</p>
                  <p className="mt-1 text-xs text-slate-600">
                    RapidBro needs your location to find nearby bus stops. Please enable location access for this site in your browser settings, then refresh the page.
                  </p>
                </>
              ) : (
                <>
                  <p className="text-sm font-semibold text-amber-900">Location access needed</p>
                  <p className="mt-1 text-xs text-slate-600">
                    RapidBro uses your location to find nearby bus stops and show real-time arrivals. Your location is only used locally and never stored.
                  </p>
                  <Button
                    type="button"
                    onClick={() => {
                      hasAutoRequestedNearestStopRef.current = true
                      void handleFindNearestStop()
                    }}
                    disabled={isLoading}
                    size="sm"
                    className="mt-3 bg-amber-500 text-slate-900 hover:bg-amber-400"
                  >
                    {isLoading ? (
                      <>
                        <LoaderCircle className="animate-spin" />
                        Getting location...
                      </>
                    ) : (
                      <>
                        <LocateFixed />
                        Allow location access
                      </>
                    )}
                  </Button>
                </>
              )}
            </div>
          </div>
        </section>
      ) : null}

      {errorMessage ? (
        <section className={panelSectionClass}>
          <p className="inline-flex items-center gap-2 text-sm font-medium text-rose-700">
            <AlertTriangle className="h-4 w-4" />
            Location Error
          </p>
          <p className="mt-1 text-xs text-slate-600">{errorMessage}</p>
        </section>
      ) : null}

      <section className={panelSectionClass}>
        <p className="inline-flex items-center gap-2 text-sm font-medium text-amber-900">
          <MapPin className="h-4 w-4" />
          Nearest Stop
        </p>

        {nearestStop ? (
          <>
            <p className="mt-2 text-base font-semibold">{nearestStop.stop_name}</p>
            <p className={panelSubtleTextClass}>Stop ID: {nearestStop.stop_id}</p>
            <p className="mt-1 text-xs text-slate-700">
              {nearestStop.stop_desc || 'No stop description'}
            </p>
            <p className="mt-2 text-xs text-slate-700">
              {nearestStop.distance_meters.toFixed(1)} m · {nearestStop.distance_km.toFixed(3)} km
            </p>
          </>
        ) : (
          <p className="mt-2 text-xs text-slate-600">Waiting for nearest stop data.</p>
        )}
      </section>

      <section className={panelSectionClass}>
        <p className="text-sm font-medium text-amber-900">Routes At This Stop</p>
        {isLoadingRoutes ? (
          <p className="mt-2 inline-flex items-center gap-2 text-xs text-slate-600">
            <LoaderCircle className="h-4 w-4 animate-spin" />
            Loading routes...
          </p>
        ) : null}
        {routeErrorMessage ? (
          <p className="mt-2 text-xs text-rose-700">{routeErrorMessage}</p>
        ) : null}
        {!isLoadingRoutes && !routeErrorMessage && stopRoutes.length === 0 ? (
          <p className="mt-2 text-xs text-slate-600">No registered routes for this stop.</p>
        ) : null}
        {!isLoadingRoutes && stopRoutes.length > 0 ? (
          <div className="mt-2 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => void handleSelectAllRoutes()}
              className={`rounded-xl border px-3 py-2 text-left text-xs transition-colors ${
                selectedMapRouteId === null
                  ? 'border-amber-300 bg-amber-200/20 text-amber-900'
                  : 'border-amber-200/80 bg-white/70 text-slate-700 hover:bg-amber-100/80'
              }`}
            >
              <p className="font-medium">All Routes</p>
              <p className="text-[11px] text-slate-600">
                Overlay every route on the map
              </p>
            </button>
            {stopRoutes.map((route) => (
              <button
                key={route.route_id}
                type="button"
                onClick={() => void handleSelectMapRoute(route.route_id)}
                className={`rounded-xl border px-3 py-2 text-left text-xs transition-colors ${
                  selectedMapRouteId === route.route_id
                    ? 'border-amber-300 bg-amber-200/20 text-amber-900'
                    : 'border-amber-200/80 bg-white/70 text-slate-700 hover:bg-amber-100/80'
                }`}
              >
                <p className="font-medium">{route.route_short_name || route.route_id}</p>
                <p className="text-[11px] text-slate-600">{route.route_long_name}</p>
              </button>
            ))}
          </div>
        ) : null}
      </section>

      <section className={panelSectionClass}>
        <p className="text-sm font-medium text-amber-900">
          Active Buses To This Stop ({nearestStopEta.length})
        </p>
        {isLoadingEta ? (
          <p className="mt-2 inline-flex items-center gap-2 text-xs text-slate-600">
            <LoaderCircle className="h-4 w-4 animate-spin" />
            Loading ETA...
          </p>
        ) : null}
        {etaErrorMessage ? <p className="mt-2 text-xs text-rose-700">{etaErrorMessage}</p> : null}
        {!isLoadingEta && !etaErrorMessage && nearestStopEta.length === 0 ? (
          <p className="mt-2 text-xs text-slate-600">No active buses right now.</p>
        ) : null}
        {!isLoadingEta && nearestStopEta.length > 0 ? (
          <div className="mt-2 space-y-2">
            {nearestStopEta.map((eta) => (
              <button
                key={getBusKey(eta)}
                type="button"
                onClick={() => handleSelectBus(eta)}
                className={`block w-full rounded-xl border p-2 text-left text-xs transition-colors ${
                  selectedBusKey === getBusKey(eta)
                    ? 'border-amber-300 bg-amber-200/20 text-amber-900'
                    : 'border-amber-200/80 bg-white/70 text-slate-800 hover:bg-amber-100/80'
                }`}
              >
                <p className="font-medium">Bus {eta.bus_no}</p>
                <p className="text-slate-700">
                  Route {eta.route_id} · ETA {eta.eta_minutes.toFixed(1)} min
                </p>
                <p className="text-slate-600">
                  {eta.stops_away} stops away · {eta.distance_km.toFixed(2)} km
                </p>
                <p className="text-slate-600">Current: {eta.current_stop_name}</p>
              </button>
            ))}
          </div>
        ) : null}
      </section>

      {selectedBus ? (
        <section className={panelSectionClass}>
          <p className="text-sm font-medium text-amber-900">
            Selected Bus {selectedBus.bus_no} · Route {selectedBus.route_id}
          </p>
          <p className="mt-1 text-xs text-slate-700">
            ETA {selectedBus.eta_minutes.toFixed(1)} min · {selectedBus.stops_away} stops away
          </p>
          <p className="text-xs text-slate-600">
            Current stop: {selectedBus.current_stop_name} ({selectedBus.current_stop_id})
            {selectedBus.stop_resolution_source === 'derived' ? ' · Estimated from GPS' : ''}
          </p>

          {isLoadingSelectedRoute ||
          isLoadingSelectedRouteShape ||
          (selectedRouteStops && !selectedRouteShape) ? (
            <p className="mt-3 inline-flex items-center gap-2 text-xs text-slate-600">
              <LoaderCircle className="h-4 w-4 animate-spin" />
              Loading route detail...
            </p>
          ) : null}

          {selectedRouteErrorMessage ? (
            <p className="mt-3 text-xs text-rose-700">{selectedRouteErrorMessage}</p>
          ) : null}

          {selectedRouteStops && selectedRouteShape ? (
            <div className="mt-3">
              <BusRouteLine
                routeShortName={
                  selectedRouteStops.route_short_name || selectedRouteStops.route_id
                }
                routeLongName={selectedRouteStops.route_long_name}
                stops={selectedRouteStops.stops}
                currentStopId={selectedBus.current_stop_id}
                currentSequence={selectedBus.current_sequence}
                targetStopId={nearestStop?.stop_id ?? null}
                targetLabel="Your selected nearest stop"
              />
            </div>
          ) : null}
        </section>
      ) : null}
    </MapPanelShell>
  )
}
