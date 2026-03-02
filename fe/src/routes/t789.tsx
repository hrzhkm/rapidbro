import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, RefreshCw } from 'lucide-react'
import type { LayerGroup, Map as LeafletMap } from 'leaflet'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { BusRouteLine } from '@/components/BusRouteLine'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Dialog } from '@/components/ui/dialog'

export const Route = createFileRoute('/t789')({
  component: T789Page,
})

type T789Bus = {
  bus_no: string
  route: string
  latitude: number
  longitude: number
  speed: number
  busstop_id?: string | null
  resolved_stop_id?: string | null
  resolved_stop_name?: string | null
  resolved_stop_sequence?: number | null
  stop_resolution_source?: 'live' | 'derived' | null
}

type BusEta = {
  route_id?: string
  bus_no: string
  current_stop_id: string
  current_stop_name: string
  current_sequence?: number
  stop_resolution_source: 'live' | 'derived'
  stops_away: number
  distance_km: number
  speed_kmh: number
  eta_minutes: number
}

type RouteStopsResponse = {
  route_id: string
  route_short_name: string
  route_long_name: string
  stops: Array<{
    stop_id: string
    stop_name: string
    stop_desc: string
    stop_lat: number
    stop_lon: number
    sequence: number
  }>
}

type RouteShapeResponse = {
  route_id: string
  shape_id: string
  points: Array<{
    lat: number
    lon: number
    sequence: number
  }>
}

type T789RouteMapProps = {
  activeBuses: T789Bus[]
  routeStops: RouteStopsResponse | null
  routeShape: RouteShapeResponse | null
  selectedBusNo: string | null
  targetStopId: string
  onSelectBus: (busNo: string) => void
}

function escapeLeafletHtml(value: string) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}

function T789RouteMap({
  activeBuses,
  routeStops,
  routeShape,
  selectedBusNo,
  targetStopId,
  onSelectBus,
}: T789RouteMapProps) {
  const mapContainerRef = useRef<HTMLDivElement | null>(null)
  const mapRef = useRef<LeafletMap | null>(null)
  const overlayLayerRef = useRef<LayerGroup | null>(null)
  const hasAutoFitRef = useRef(false)
  const lastSelectedBusNoRef = useRef<string | null>(null)

  useEffect(() => {
    let isDisposed = false

    const setupMap = async () => {
      if (
        isDisposed ||
        typeof window === 'undefined' ||
        !mapContainerRef.current ||
        mapRef.current
      ) {
        return
      }

      const L = await import('leaflet')

      if (isDisposed || !mapContainerRef.current || mapRef.current) {
        return
      }

      const map = L.map(mapContainerRef.current, {
        zoomControl: true,
        attributionControl: true,
      })

      L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
        attribution: '&copy; OpenStreetMap contributors',
      }).addTo(map)

      mapRef.current = map
      overlayLayerRef.current = L.layerGroup().addTo(map)
      requestAnimationFrame(() => {
        map.invalidateSize()
      })
    }

    void setupMap()

    return () => {
      isDisposed = true
      overlayLayerRef.current?.clearLayers()
      mapRef.current?.remove()
      overlayLayerRef.current = null
      mapRef.current = null
    }
  }, [])

  useEffect(() => {
    let isDisposed = false

    const syncMap = async () => {
      if (!mapRef.current) {
        return
      }

      const L = await import('leaflet')

      if (isDisposed || !mapRef.current) {
        return
      }

      const map = mapRef.current
      const overlayLayer = overlayLayerRef.current ?? L.layerGroup().addTo(map)

      overlayLayerRef.current = overlayLayer
      overlayLayer.clearLayers()

      const points: Array<[number, number]> = []
      const stops = routeStops?.stops ?? []
      const shapePoints = routeShape?.points ?? []
      const targetStop =
        stops.find((stop) => stop.stop_id === targetStopId) ?? null

      if (shapePoints.length > 1) {
        const routePath = shapePoints.map(
          (point) => [point.lat, point.lon] as [number, number],
        )
        routePath.forEach((point) => points.push(point))

        L.polyline(routePath, {
          color: '#0f766e',
          weight: 5,
          opacity: 0.9,
        }).addTo(overlayLayer)
      } else if (stops.length > 1) {
        const routePath = stops.map(
          (stop) => [stop.stop_lat, stop.stop_lon] as [number, number],
        )
        routePath.forEach((point) => points.push(point))

        L.polyline(routePath, {
          color: '#0f766e',
          weight: 4,
          opacity: 0.8,
        }).addTo(overlayLayer)
      }

      stops.forEach((stop) => {
        const isTargetStop = stop.stop_id === targetStopId

        points.push([stop.stop_lat, stop.stop_lon])

        L.circleMarker([stop.stop_lat, stop.stop_lon], {
          radius: isTargetStop ? 7 : 4,
          color: isTargetStop ? '#b45309' : '#475569',
          weight: 2,
          fillColor: isTargetStop ? '#f59e0b' : '#f8fafc',
          fillOpacity: 0.95,
        })
          .bindTooltip(
            isTargetStop
              ? `${stop.stop_name} (KL Gateway target)`
              : stop.stop_name,
          )
          .addTo(overlayLayer)
      })

      activeBuses.forEach((bus) => {
        const isSelected = bus.bus_no === selectedBusNo
        const currentStopLabel =
          bus.resolved_stop_name ??
          bus.resolved_stop_id ??
          bus.busstop_id ??
          'Unknown'

        points.push([bus.latitude, bus.longitude])

        const marker = L.marker([bus.latitude, bus.longitude], {
          icon: L.divIcon({
            className: 't789-bus-icon',
            html: `<div class="t789-bus-marker${isSelected ? ' is-selected' : ''}"><img src="/bus-icon.svg" alt="" class="t789-bus-marker-image" /></div>`,
            iconSize: [34, 34],
            iconAnchor: [17, 17],
          }),
          title: `Bus ${bus.bus_no}`,
        })

        marker.on('click', () => {
          onSelectBus(bus.bus_no)
        })

        marker
          .bindTooltip(
            `<strong>Bus ${escapeLeafletHtml(bus.bus_no)}</strong><br/>Speed: ${bus.speed.toFixed(1)} km/h<br/>Current stop: ${escapeLeafletHtml(currentStopLabel)}`,
          )
          .bindPopup(
            `<strong>Bus ${escapeLeafletHtml(bus.bus_no)}</strong><br/>Speed: ${bus.speed.toFixed(1)} km/h<br/>Current stop: ${escapeLeafletHtml(currentStopLabel)}`,
          )
          .addTo(overlayLayer)
      })

      if (targetStop && activeBuses.length === 0) {
        points.push([targetStop.stop_lat, targetStop.stop_lon])
      }

      const selectedBus =
        selectedBusNo === null
          ? null
          : (activeBuses.find((bus) => bus.bus_no === selectedBusNo) ?? null)

      if (selectedBus) {
        if (lastSelectedBusNoRef.current !== selectedBus.bus_no) {
          map.flyTo([selectedBus.latitude, selectedBus.longitude], 15, {
            animate: true,
            duration: 0.4,
          })
          lastSelectedBusNoRef.current = selectedBus.bus_no
        }
      } else {
        lastSelectedBusNoRef.current = null

        if (!hasAutoFitRef.current && points.length > 0) {
          map.fitBounds(points, {
            padding: [24, 24],
            animate: false,
          })
          hasAutoFitRef.current = true
        }
      }

      requestAnimationFrame(() => {
        map.invalidateSize()
      })
    }

    void syncMap()

    return () => {
      isDisposed = true
    }
  }, [
    activeBuses,
    onSelectBus,
    routeShape,
    routeStops,
    selectedBusNo,
    targetStopId,
  ])

  return (
    <div className="space-y-2">
      <div
        ref={mapContainerRef}
        className="t789-map rounded-xl border"
        aria-label="Live T789 route map"
      />
      <p className="text-xs text-muted-foreground">
        The route line, KL Gateway target stop, and active buses are shown on
        the map.
      </p>
    </div>
  )
}

function T789Page() {
  const targetStopId = '1000838'
  const apiBaseUrl = useMemo(
    () => import.meta.env.VITE_BE_URL ?? 'http://localhost:3030',
    [],
  )

  const [activeBuses, setActiveBuses] = useState<T789Bus[]>([])
  const [etas, setEtas] = useState<BusEta[]>([])
  const [routeStops, setRouteStops] = useState<RouteStopsResponse | null>(null)
  const [routeShape, setRouteShape] = useState<RouteShapeResponse | null>(null)
  const [stopNameById, setStopNameById] = useState<Record<string, string>>({})
  const [selectedBusNo, setSelectedBusNo] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [etaErrorMessage, setEtaErrorMessage] = useState<string | null>(null)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)

  const selectedActiveBus =
    selectedBusNo === null
      ? null
      : (activeBuses.find((bus) => bus.bus_no === selectedBusNo) ?? null)
  const selectedEta =
    selectedBusNo === null
      ? null
      : (etas.find((eta) => eta.bus_no === selectedBusNo) ?? null)
  const targetStopName = stopNameById[targetStopId] || 'KL Gateway'
  const selectedCurrentStopId =
    selectedActiveBus?.resolved_stop_id ??
    selectedActiveBus?.busstop_id ??
    selectedEta?.current_stop_id ??
    null
  const selectedCurrentSequence =
    selectedActiveBus?.resolved_stop_sequence ??
    routeStops?.stops.find((stop) => stop.stop_id === selectedCurrentStopId)
      ?.sequence ??
    selectedEta?.current_sequence ??
    null
  const interactiveCardClassName =
    'block w-full cursor-pointer rounded border p-2 text-left transition-colors outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]'
  const handleSelectBus = useCallback((busNo: string) => {
    setSelectedBusNo(busNo)
  }, [])

  const normalizeT789Buses = (payload: unknown): T789Bus[] => {
    if (Array.isArray(payload)) {
      return payload as T789Bus[]
    }

    if (payload && typeof payload === 'object' && 'bus_no' in payload) {
      return [payload as T789Bus]
    }

    return []
  }

  const fetchT789Buses = useCallback(async () => {
    setErrorMessage(null)
    setEtaErrorMessage(null)
    setIsLoading(true)

    try {
      const [busesResponse, etaResponse, stopsResponse, shapeResponse] =
        await Promise.all([
          fetch(`${apiBaseUrl}/get-route-t789`),
          fetch(`${apiBaseUrl}/get-t789-eta`),
          fetch(`${apiBaseUrl}/route/T7890/stops`),
          fetch(`${apiBaseUrl}/route/T7890/shape`),
        ])

      if (!busesResponse.ok) {
        const fallbackMessage = 'Unable to fetch active T789 buses'
        const body = (await busesResponse.json().catch(() => null)) as {
          error?: string
        } | null
        throw new Error(body?.error ?? fallbackMessage)
      }

      const payload = (await busesResponse.json()) as unknown
      const normalizedBuses = normalizeT789Buses(payload)
      setActiveBuses(normalizedBuses)
      setSelectedBusNo((current) => {
        if (current && normalizedBuses.some((bus) => bus.bus_no === current)) {
          return current
        }

        return null
      })

      if (etaResponse.ok) {
        const etaData = (await etaResponse.json()) as BusEta[]
        setEtas(etaData)
      } else {
        const fallbackMessage = 'Unable to fetch ETA to KL Gateway'
        const body = (await etaResponse.json().catch(() => null)) as {
          error?: string
        } | null
        setEtaErrorMessage(body?.error ?? fallbackMessage)
        setEtas([])
      }

      if (stopsResponse.ok) {
        const stopsData = (await stopsResponse.json()) as RouteStopsResponse
        setRouteStops(stopsData)
        const nameMap = stopsData.stops.reduce<Record<string, string>>(
          (acc, stop) => {
            acc[stop.stop_id] = stop.stop_name
            return acc
          },
          {},
        )
        setStopNameById(nameMap)
      }

      if (shapeResponse.ok) {
        const shapeData = (await shapeResponse.json()) as RouteShapeResponse
        setRouteShape(shapeData)
      } else {
        setRouteShape(null)
      }

      setLastUpdated(new Date())
    } catch (error) {
      setErrorMessage(
        error instanceof Error
          ? error.message
          : 'Unable to fetch active T789 buses',
      )
    } finally {
      setIsLoading(false)
    }
  }, [apiBaseUrl])

  useEffect(() => {
    fetchT789Buses()

    const id = setInterval(() => {
      fetchT789Buses()
    }, 15000)

    return () => clearInterval(id)
  }, [fetchT789Buses])

  return (
    <main className="mx-auto max-w-4xl p-4 md:p-6">
      <Card>
        <CardHeader>
          <CardTitle>T789 Route</CardTitle>
          <CardDescription>Active buses for T789.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-3">
            <Button type="button" onClick={fetchT789Buses} disabled={isLoading}>
              {isLoading ? (
                <>
                  <LoaderCircle className="animate-spin" />
                  Refreshing...
                </>
              ) : (
                <>
                  <RefreshCw />
                  Refresh
                </>
              )}
            </Button>
            <p className="text-sm text-muted-foreground">
              {lastUpdated
                ? `Last updated: ${lastUpdated.toLocaleTimeString()}`
                : 'No updates yet'}
            </p>
          </div>

          <div className="rounded-md border p-3">
            <p className="mb-2 text-sm font-medium">Live route map</p>
            <T789RouteMap
              activeBuses={activeBuses}
              routeShape={routeShape}
              routeStops={routeStops}
              selectedBusNo={selectedBusNo}
              targetStopId={targetStopId}
              onSelectBus={handleSelectBus}
            />
          </div>

          {errorMessage ? (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3">
              <p className="inline-flex items-center gap-2 text-sm font-medium text-destructive">
                <AlertTriangle className="h-4 w-4" />
                Error
              </p>
              <p className="mt-1 text-sm text-muted-foreground">
                {errorMessage}
              </p>
            </div>
          ) : null}

          {etaErrorMessage ? (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3">
              <p className="inline-flex items-center gap-2 text-sm font-medium text-destructive">
                <AlertTriangle className="h-4 w-4" />
                ETA Error
              </p>
              <p className="mt-1 text-sm text-muted-foreground">
                {etaErrorMessage}
              </p>
            </div>
          ) : null}

          <div className="rounded-md border p-3">
            <p className="mb-2 text-sm font-medium">
              All active T789 buses ({activeBuses.length})
            </p>
            {!isLoading && !errorMessage && activeBuses.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No active T789 buses right now.
              </p>
            ) : null}

            {activeBuses.length > 0 ? (
              <div className="space-y-2">
                {activeBuses.map((bus) => (
                  <button
                    key={`${bus.route}-${bus.bus_no}`}
                    type="button"
                    onClick={() => handleSelectBus(bus.bus_no)}
                    className={`${interactiveCardClassName} ${
                      selectedBusNo === bus.bus_no
                        ? 'border-foreground bg-secondary'
                        : 'hover:bg-muted/40 active:bg-muted/60'
                    }`}
                  >
                    <p className="font-medium">
                      Bus {bus.bus_no} · Route {bus.route}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {bus.latitude.toFixed(5)}, {bus.longitude.toFixed(5)} ·{' '}
                      {bus.speed.toFixed(1)} km/h
                    </p>
                    <p className="text-sm text-muted-foreground">
                      Current stop:{' '}
                      {bus.resolved_stop_name ??
                        (bus.busstop_id
                          ? stopNameById[bus.busstop_id] || bus.busstop_id
                          : bus.resolved_stop_id) ??
                        'Unknown'}
                    </p>
                    {bus.stop_resolution_source === 'derived' ? (
                      <p className="text-sm text-muted-foreground">
                        Estimated from GPS
                      </p>
                    ) : null}
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <div className="rounded-md border p-3">
            <p className="mb-2 text-sm font-medium">ETA to KL Gateway</p>
            {!isLoading && !etaErrorMessage && etas.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No ETA is available for KL Gateway right now.
              </p>
            ) : null}

            {etas.length > 0 ? (
              <div className="space-y-2">
                {etas.map((eta) => (
                  <button
                    key={`${eta.route_id || 'T7890'}-${eta.bus_no}-${eta.current_stop_id}`}
                    type="button"
                    onClick={() => handleSelectBus(eta.bus_no)}
                    className={`${interactiveCardClassName} ${
                      selectedBusNo === eta.bus_no
                        ? 'border-foreground bg-secondary'
                        : 'hover:bg-muted/40 active:bg-muted/60'
                    }`}
                  >
                    <p className="font-medium">
                      Bus {eta.bus_no} · Route {eta.route_id || 'T7890'}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      ETA {eta.eta_minutes.toFixed(1)} min · {eta.stops_away}{' '}
                      stops away · {eta.distance_km.toFixed(2)} km
                    </p>
                    <p className="text-sm text-muted-foreground">
                      Current stop: {eta.current_stop_name}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      Stop ID: {eta.current_stop_id}
                      {eta.stop_resolution_source === 'derived'
                        ? ' · Estimated from GPS'
                        : ''}
                    </p>
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        </CardContent>
      </Card>

      <Dialog
        open={selectedBusNo !== null}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedBusNo(null)
          }
        }}
        title={
          selectedActiveBus
            ? `Bus ${selectedActiveBus.bus_no} · Route ${selectedActiveBus.route}`
            : 'T789 bus detail'
        }
        description="The current bus position and the KL Gateway target stop are highlighted on the route line."
      >
        {selectedActiveBus ? (
          <div className="space-y-4">
            {selectedEta ? (
              <div className="rounded-md border bg-muted/30 p-3 text-sm">
                <p className="font-medium">
                  ETA to {targetStopName}: {selectedEta.eta_minutes.toFixed(1)}{' '}
                  min
                </p>
                <p className="text-muted-foreground">
                  {selectedEta.stops_away} stops away ·{' '}
                  {selectedEta.distance_km.toFixed(2)} km remaining
                </p>
                <p className="text-muted-foreground">
                  Current stop: {selectedEta.current_stop_name}
                  {selectedEta.stop_resolution_source === 'derived'
                    ? ' · Estimated from GPS'
                    : ''}
                </p>
              </div>
            ) : (
              <div className="rounded-md border bg-muted/30 p-3 text-sm text-muted-foreground">
                No ETA to {targetStopName} is available for this bus right now.
              </div>
            )}

            {routeStops ? (
              <BusRouteLine
                routeShortName={routeStops.route_short_name}
                routeLongName={routeStops.route_long_name}
                stops={routeStops.stops}
                currentStopId={selectedCurrentStopId}
                currentSequence={selectedCurrentSequence}
                targetStopId={targetStopId}
                targetLabel="KL Gateway target stop"
              />
            ) : (
              <p className="text-sm text-muted-foreground">
                Route line is unavailable right now.
              </p>
            )}
          </div>
        ) : null}
      </Dialog>
    </main>
  )
}
