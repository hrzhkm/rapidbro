import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { BusRouteMap } from '@/components/BusRouteMap'
import { BusRouteLine } from '@/components/BusRouteLine'
import {
  buildRoutePolylinePoints,
  type RouteShape,
} from '@/lib/route-geometry'
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
}

type BusEta = {
  route_id?: string
  bus_no: string
  current_stop_id: string
  current_sequence?: number
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

function T789Page() {
  const targetStopId = '1000838'
  const apiBaseUrl = useMemo(
    () => import.meta.env.VITE_BE_URL ?? 'http://localhost:3030',
    [],
  )

  const [activeBuses, setActiveBuses] = useState<T789Bus[]>([])
  const [etas, setEtas] = useState<BusEta[]>([])
  const [routeStops, setRouteStops] = useState<RouteStopsResponse | null>(null)
  const [routeShape, setRouteShape] = useState<RouteShape | null>(null)
  const [stopNameById, setStopNameById] = useState<Record<string, string>>({})
  const [selectedBusNo, setSelectedBusNo] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [etaErrorMessage, setEtaErrorMessage] = useState<string | null>(null)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)

  const selectedActiveBus =
    selectedBusNo === null
      ? null
      : activeBuses.find((bus) => bus.bus_no === selectedBusNo) ?? null
  const selectedEta =
    selectedBusNo === null
      ? null
      : etas.find((eta) => eta.bus_no === selectedBusNo) ?? null
  const targetStopName = stopNameById[targetStopId] || 'KL Gateway'
  const selectedCurrentStopId =
    selectedActiveBus?.busstop_id ?? selectedEta?.current_stop_id ?? null
  const selectedCurrentSequence =
    routeStops?.stops.find((stop) => stop.stop_id === selectedCurrentStopId)
      ?.sequence ??
    selectedEta?.current_sequence ??
    null
  const routePolylinePoints = buildRoutePolylinePoints(
    routeShape,
    routeStops?.stops,
  )
  const interactiveCardClassName =
    'block w-full cursor-pointer rounded border p-2 text-left transition-colors outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]'

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
      const [busesResponse, etaResponse, stopsResponse] = await Promise.all([
        fetch(`${apiBaseUrl}/get-route-t789`),
        fetch(`${apiBaseUrl}/get-t789-eta`),
        fetch(`${apiBaseUrl}/route/T7890/stops`),
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
        if (
          current &&
          normalizedBuses.some((bus) => bus.bus_no === current)
        ) {
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

  const fetchT789Shape = useCallback(async () => {
    const response = await fetch(
      `${apiBaseUrl}/route/T7890/shape?stop_id=${encodeURIComponent(targetStopId)}`,
    )
    if (!response.ok) {
      return
    }

    const shapeData = (await response.json()) as RouteShape
    setRouteShape(shapeData)
  }, [apiBaseUrl, targetStopId])

  useEffect(() => {
    fetchT789Buses()
    void fetchT789Shape()

    const id = setInterval(() => {
      fetchT789Buses()
    }, 15000)

    return () => clearInterval(id)
  }, [fetchT789Buses, fetchT789Shape])

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
                    onClick={() => setSelectedBusNo(bus.bus_no)}
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
                      {bus.busstop_id
                        ? stopNameById[bus.busstop_id] || bus.busstop_id
                        : 'Unknown'}
                    </p>
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
                    onClick={() => setSelectedBusNo(eta.bus_no)}
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
                      Current stop:{' '}
                      {stopNameById[eta.current_stop_id] || eta.current_stop_id}
                    </p>
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <div className="rounded-md border p-3">
            <p className="mb-2 text-sm font-medium">Live route map</p>
            {routeStops ? (
              <BusRouteMap
                stops={routeStops.stops}
                polylinePoints={routePolylinePoints}
                buses={activeBuses.map((bus) => ({
                  id: `${bus.route}-${bus.bus_no}`,
                  label: `Bus ${bus.bus_no}`,
                  lat: bus.latitude,
                  lon: bus.longitude,
                  isSelected: selectedBusNo === bus.bus_no,
                }))}
                currentStopId={selectedCurrentStopId}
                targetStopId={targetStopId}
              />
            ) : (
              <p className="text-sm text-muted-foreground">
                Route map is unavailable right now.
              </p>
            )}
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
              </div>
            ) : (
              <div className="rounded-md border bg-muted/30 p-3 text-sm text-muted-foreground">
                No ETA to {targetStopName} is available for this bus right now.
              </div>
            )}

            {routeStops ? (
              <>
                <BusRouteMap
                  stops={routeStops.stops}
                  polylinePoints={routePolylinePoints}
                  buses={activeBuses.map((bus) => ({
                    id: `${bus.route}-${bus.bus_no}`,
                    label: `Bus ${bus.bus_no}`,
                    lat: bus.latitude,
                    lon: bus.longitude,
                    isSelected: selectedBusNo === bus.bus_no,
                  }))}
                  currentStopId={selectedCurrentStopId}
                  targetStopId={targetStopId}
                />
                <BusRouteLine
                  routeShortName={routeStops.route_short_name}
                  routeLongName={routeStops.route_long_name}
                  stops={routeStops.stops}
                  currentStopId={selectedCurrentStopId}
                  currentSequence={selectedCurrentSequence}
                  targetStopId={targetStopId}
                  targetLabel="KL Gateway target stop"
                />
              </>
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
