import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, LocateFixed, MapPin } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

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

type UserCoords = {
  lat: number
  lon: number
}

function App() {
  const apiBaseUrl = useMemo(
    () => import.meta.env.VITE_BE_URL ?? 'http://localhost:3030',
    [],
  )
  const [coords, setCoords] = useState<UserCoords | null>(null)
  const [nearestStop, setNearestStop] = useState<NearestStopResponse | null>(
    null,
  )
  const [nearestStopEta, setNearestStopEta] = useState<BusEta[]>([])
  const [stopRoutes, setStopRoutes] = useState<StopRouteSummary[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [isLoadingEta, setIsLoadingEta] = useState(false)
  const [isLoadingRoutes, setIsLoadingRoutes] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [etaErrorMessage, setEtaErrorMessage] = useState<string | null>(null)
  const [routeErrorMessage, setRouteErrorMessage] = useState<string | null>(
    null,
  )

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

  const handleFindNearestStop = () => {
    setErrorMessage(null)
    setEtaErrorMessage(null)
    setRouteErrorMessage(null)
    setNearestStop(null)
    setNearestStopEta([])
    setStopRoutes([])
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
                Find nearest stop
              </>
            )}
          </Button>

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
                      <div
                        key={`${eta.route_id}-${eta.bus_no}-${eta.current_stop_id}`}
                        className="rounded border bg-background p-2 text-sm"
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
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            </div>
          ) : (
            <div className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              Tap{' '}
              <span className="font-medium text-foreground">
                Find nearest stop
              </span>{' '}
              to begin.
            </div>
          )}
        </CardContent>
      </Card>
    </main>
  )
}
