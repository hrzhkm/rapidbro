import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, LocateFixed, MapPin } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
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
  current_sequence: number
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

function App() {
  const apiBaseUrl = useMemo(
    () => import.meta.env.VITE_BE_URL ?? 'http://localhost:3030',
    [],
  )
  const hasAutoRequestedNearestStopRef = useRef(false)
  const [coords, setCoords] = useState<UserCoords | null>(null)
  const [nearestStop, setNearestStop] = useState<NearestStopResponse | null>(
    null,
  )
  const [nearestStopEta, setNearestStopEta] = useState<BusEta[]>([])
  const [stopRoutes, setStopRoutes] = useState<StopRouteSummary[]>([])
  const [routeStopsByRoute, setRouteStopsByRoute] = useState<
    Record<string, RouteStopsResponse>
  >({})
  const [selectedBusKey, setSelectedBusKey] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [isLoadingEta, setIsLoadingEta] = useState(false)
  const [isLoadingRoutes, setIsLoadingRoutes] = useState(false)
  const [isLoadingSelectedRoute, setIsLoadingSelectedRoute] = useState(false)
  const [lastFetchedAt, setLastFetchedAt] = useState<Date | null>(null)
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

  const fetchNearestStop = async (lat: number, lon: number) => {
    const params = new URLSearchParams({
      lat: lat.toString(),
      lon: lon.toString(),
    })
    const response = await fetch(
      `${apiBaseUrl}/stops/nearest?${params.toString()}`,
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

  const fetchEtaToStop = async (stopId: string) => {
    setEtaErrorMessage(null)
    setNearestStopEta([])
    setSelectedBusKey(null)
    setSelectedRouteErrorMessage(null)
    setIsLoadingEta(true)

    try {
      const response = await fetch(
        `${apiBaseUrl}/stops/${encodeURIComponent(stopId)}/eta`,
      )
      if (!response.ok) {
        const fallbackMessage = 'Unable to fetch ETA for nearest stop'
        const body = (await response.json().catch(() => null)) as {
          error?: string
        } | null
        throw new Error(body?.error ?? fallbackMessage)
      }

      const data = (await response.json()) as BusEta[]
      setNearestStopEta(data)
      setSelectedBusKey(null)
    } catch (error) {
      setEtaErrorMessage(
        error instanceof Error
          ? error.message
          : 'Unable to fetch ETA for nearest stop',
      )
    } finally {
      setIsLoadingEta(false)
    }
  }

  const fetchRoutesForStop = async (stopId: string) => {
    setRouteErrorMessage(null)
    setStopRoutes([])
    setIsLoadingRoutes(true)

    try {
      const response = await fetch(
        `${apiBaseUrl}/stops/${encodeURIComponent(stopId)}/routes`,
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

  const fetchRouteStops = async (routeId: string) => {
    if (routeStopsByRoute[routeId]) {
      return routeStopsByRoute[routeId]
    }

    setSelectedRouteErrorMessage(null)
    setIsLoadingSelectedRoute(true)

    try {
      const response = await fetch(
        `${apiBaseUrl}/route/${encodeURIComponent(routeId)}/stops`,
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
      setSelectedRouteErrorMessage(message)
      throw error
    } finally {
      setIsLoadingSelectedRoute(false)
    }
  }

  const handleSelectBus = async (eta: BusEta) => {
    setSelectedBusKey(getBusKey(eta))
    setSelectedRouteErrorMessage(null)

    if (routeStopsByRoute[eta.route_id]) {
      return
    }

    try {
      await fetchRouteStops(eta.route_id)
    } catch {
      // Error state is handled above so the selected bus card can still render.
    }
  }

  const handleFindNearestStop = () => {
    setErrorMessage(null)
    setEtaErrorMessage(null)
    setRouteErrorMessage(null)
    setSelectedRouteErrorMessage(null)
    setNearestStop(null)
    setNearestStopEta([])
    setStopRoutes([])
    setRouteStopsByRoute({})
    setSelectedBusKey(null)
    setIsLoading(true)

    if (!('geolocation' in navigator)) {
      setIsLoading(false)
      setErrorMessage('Geolocation is not supported by this browser.')
      return
    }

    navigator.geolocation.getCurrentPosition(
      async (position) => {
        const lat = position.coords.latitude
        const lon = position.coords.longitude
        setCoords({ lat, lon })

        try {
          const nearestStopData = await fetchNearestStop(lat, lon)
          await Promise.all([
            fetchEtaToStop(nearestStopData.stop_id),
            fetchRoutesForStop(nearestStopData.stop_id),
          ])
          setLastFetchedAt(new Date())
        } catch (error) {
          setErrorMessage(
            error instanceof Error
              ? error.message
              : 'Unable to fetch nearest bus stop',
          )
        } finally {
          setIsLoading(false)
        }
      },
      (error) => {
        setIsLoading(false)
        setErrorMessage(error.message || 'Unable to read your location.')
      },
      {
        enableHighAccuracy: true,
        timeout: 10000,
        maximumAge: 30000,
      },
    )
  }

  useEffect(() => {
    if (hasAutoRequestedNearestStopRef.current) {
      return
    }

    hasAutoRequestedNearestStopRef.current = true
    handleFindNearestStop()
  }, [])

  useEffect(() => {
    const id = window.setInterval(() => {
      handleFindNearestStop()
    }, 30000)

    return () => window.clearInterval(id)
  }, [])

  return (
    <main className="mx-auto max-w-4xl p-4 md:p-6">
      <Card className="mb-6">
        <CardHeader>
          <CardTitle>Nearest Bus Stop Finder</CardTitle>
          <CardDescription>
            Get your current coordinates from the browser, then fetch the
            closest GTFS stop from the backend.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-3">
            <Button
              type="button"
              onClick={handleFindNearestStop}
              disabled={isLoading}
            >
              {isLoading ? (
                <>
                  <LoaderCircle className="animate-spin" />
                  Finding...
                </>
              ) : (
                <>
                  <LocateFixed />
                  {coords ? 'Refresh nearest stop' : 'Find nearest stop'}
                </>
              )}
            </Button>
            <p className="text-sm text-muted-foreground">
              {lastFetchedAt
                ? `Last fetched: ${lastFetchedAt.toLocaleTimeString()}`
                : 'No fetch yet'}
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

          {coords ? (
            <div className="rounded-md border bg-muted/40 p-3">
              <p className="text-sm font-medium">Your location</p>
              <p className="mt-1 text-sm text-muted-foreground">
                Latitude: {coords.lat.toFixed(6)} | Longitude:{' '}
                {coords.lon.toFixed(6)}
              </p>
            </div>
          ) : null}

          {nearestStop ? (
            <div className="rounded-md border p-4">
              <p className="inline-flex items-center gap-2 text-sm font-medium">
                <MapPin className="h-4 w-4" />
                Nearest stop
              </p>
              <p className="mt-2 text-lg font-semibold">
                {nearestStop.stop_name}
              </p>
              <p className="text-sm text-muted-foreground">
                Stop ID: {nearestStop.stop_id}
              </p>
              <p className="mt-1 text-sm text-muted-foreground">
                {nearestStop.stop_desc || 'No stop description'}
              </p>
              <p className="mt-2 text-sm">
                Distance: {nearestStop.distance_meters.toFixed(1)} m (
                {nearestStop.distance_km.toFixed(3)} km)
              </p>

              <div className="mt-4 rounded-md border bg-muted/30 p-3">
                <p className="text-sm font-medium">
                  Routes serving this stop
                </p>
                {isLoadingRoutes ? (
                  <p className="mt-2 inline-flex items-center gap-2 text-sm text-muted-foreground">
                    <LoaderCircle className="h-4 w-4 animate-spin" />
                    Loading routes...
                  </p>
                ) : null}

                {routeErrorMessage ? (
                  <p className="mt-2 text-sm text-destructive">
                    {routeErrorMessage}
                  </p>
                ) : null}

                {!isLoadingRoutes &&
                !routeErrorMessage &&
                stopRoutes.length === 0 ? (
                  <p className="mt-2 text-sm text-muted-foreground">
                    No registered routes found for this stop.
                  </p>
                ) : null}

                {!isLoadingRoutes && stopRoutes.length > 0 ? (
                  <div className="mt-2 flex flex-wrap gap-2">
                    {stopRoutes.map((route) => (
                      <div
                        key={route.route_id}
                        className="rounded-md border bg-background px-3 py-2 text-sm"
                      >
                        <p className="font-medium">
                          {route.route_short_name || route.route_id}
                        </p>
                        <p className="text-muted-foreground">
                          {route.route_long_name}
                        </p>
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>

              <div className="mt-4 rounded-md border bg-muted/30 p-3">
                <p className="text-sm font-medium">
                  Active buses heading to this stop ({nearestStopEta.length})
                </p>
                {isLoadingEta ? (
                  <p className="mt-2 inline-flex items-center gap-2 text-sm text-muted-foreground">
                    <LoaderCircle className="h-4 w-4 animate-spin" />
                    Loading ETA...
                  </p>
                ) : null}

                {etaErrorMessage ? (
                  <p className="mt-2 text-sm text-destructive">
                    {etaErrorMessage}
                  </p>
                ) : null}

                {!isLoadingEta &&
                !etaErrorMessage &&
                nearestStopEta.length === 0 ? (
                  <p className="mt-2 text-sm text-muted-foreground">
                    No active buses heading to this stop right now.
                  </p>
                ) : null}

                {!isLoadingEta && nearestStopEta.length > 0 ? (
                  <div className="mt-2 space-y-2">
                    {nearestStopEta.map((eta) => (
                      <button
                        key={getBusKey(eta)}
                        type="button"
                        onClick={() => handleSelectBus(eta)}
                        className={`block w-full rounded border bg-background p-2 text-left text-sm transition-colors ${
                          selectedBusKey === getBusKey(eta)
                            ? 'border-foreground bg-secondary'
                            : 'hover:bg-muted/40'
                        }`}
                      >
                        <p className="font-medium">Bus {eta.bus_no}</p>
                        <p className="text-muted-foreground">
                          Route {eta.route_id} · ETA{' '}
                          {eta.eta_minutes.toFixed(1)} min · {eta.stops_away}{' '}
                          stops away · {eta.distance_km.toFixed(2)} km
                        </p>
                        <p className="text-muted-foreground">
                          Current stop ID: {eta.current_stop_id}
                        </p>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>

            </div>
          ) : (
            <div className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              Automatically checking your location. You can also tap{' '}
              <span className="font-medium text-foreground">
                {coords ? 'Refresh nearest stop' : 'Find nearest stop'}
              </span>{' '}
              any time.
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog
        open={selectedBus !== null}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedBusKey(null)
          }
        }}
        title={
          selectedBus
            ? `Bus ${selectedBus.bus_no} · Route ${selectedBus.route_id}`
            : 'Bus route detail'
        }
        description="The current bus position and your selected stop are highlighted on the route line."
      >
        {selectedBus ? (
          <div className="space-y-4">
            <div className="rounded-md border bg-muted/30 p-3 text-sm">
              <p className="font-medium">
                ETA {selectedBus.eta_minutes.toFixed(1)} min
              </p>
              <p className="text-muted-foreground">
                {selectedBus.stops_away} stops away ·{' '}
                {selectedBus.distance_km.toFixed(2)} km remaining
              </p>
              <p className="text-muted-foreground">
                Current stop ID: {selectedBus.current_stop_id}
              </p>
            </div>

            {isLoadingSelectedRoute ? (
              <p className="inline-flex items-center gap-2 text-sm text-muted-foreground">
                <LoaderCircle className="h-4 w-4 animate-spin" />
                Loading route line...
              </p>
            ) : null}

            {selectedRouteErrorMessage ? (
              <p className="text-sm text-destructive">
                {selectedRouteErrorMessage}
              </p>
            ) : null}

            {selectedRouteStops ? (
              <BusRouteLine
                routeShortName={
                  selectedRouteStops.route_short_name ||
                  selectedRouteStops.route_id
                }
                routeLongName={selectedRouteStops.route_long_name}
                stops={selectedRouteStops.stops}
                currentStopId={selectedBus.current_stop_id}
                currentSequence={selectedBus.current_sequence}
                targetStopId={nearestStop?.stop_id ?? null}
                targetLabel="Your selected nearest stop"
              />
            ) : null}
          </div>
        ) : null}
      </Dialog>
    </main>
  )
}
