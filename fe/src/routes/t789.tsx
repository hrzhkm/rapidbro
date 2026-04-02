import { createFileRoute } from '@tanstack/react-router'
import { AlertTriangle, LoaderCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { BusRouteMap } from '@/components/BusRouteMap'
import { BusRouteLine } from '@/components/BusRouteLine'
import { MapPanelShell } from '@/components/MapPanelShell'
import { Button } from '@/components/ui/button'
import {
  buildRoutePolylinePoints,
  type RouteShape,
} from '@/lib/route-geometry'

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

const panelSectionClass =
  'rounded-2xl border border-amber-200/80 bg-white/78 p-3 text-slate-800 shadow-[inset_0_1px_0_rgba(255,255,255,0.55)]'

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
  const [isLoadingRouteShape, setIsLoadingRouteShape] = useState(false)
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
    'block w-full cursor-pointer rounded-xl border border-amber-200/80 bg-white/70 p-2 text-left text-xs transition-colors outline-none focus-visible:border-amber-300 focus-visible:ring-2 focus-visible:ring-amber-200/60'

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
    setIsLoadingRouteShape(true)
    try {
      const response = await fetch(
        `${apiBaseUrl}/route/T7890/shape?stop_id=${encodeURIComponent(targetStopId)}`,
      )
      if (!response.ok) {
        return
      }

      const shapeData = (await response.json()) as RouteShape
      setRouteShape(shapeData)
    } finally {
      setIsLoadingRouteShape(false)
    }
  }, [apiBaseUrl, targetStopId])

  useEffect(() => {
    void fetchT789Buses()
    void fetchT789Shape()

    const id = setInterval(() => {
      void fetchT789Buses()
    }, 15000)

    return () => clearInterval(id)
  }, [fetchT789Buses, fetchT789Shape])

  const mapContent =
    routeStops && routeShape ? (
      <BusRouteMap
        className="h-full"
        fullScreen
        showLegend={false}
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
      <div className="flex h-full items-center justify-center bg-[radial-gradient(circle_at_20%_12%,rgba(251,191,36,0.42),transparent_44%),radial-gradient(circle_at_84%_82%,rgba(34,211,238,0.22),transparent_38%),linear-gradient(115deg,#fff7ed_0%,#fffbeb_52%,#fef3c7_100%)] px-6 text-center text-slate-800">
        <div>
          <p className="font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-lg font-semibold tracking-tight text-amber-900">
            T789 Live Route
          </p>
          <p className="mt-2 text-sm text-slate-700">
            {isLoadingRouteShape
              ? 'Loading route shape...'
              : 'Waiting for route and bus data to render the map.'}
          </p>
        </div>
      </div>
    )

  return (
    <MapPanelShell
      map={mapContent}
      panelTitle="T789 Control Panel"
      panelDescription="Monitor active T789 buses and ETA to KL Gateway."
      panelStatus={
        <p className="text-slate-600">
          {lastUpdated
            ? `Updated ${lastUpdated.toLocaleTimeString()}`
            : 'No updates yet'}
        </p>
      }
      panelActions={
        <Button
          type="button"
          onClick={() => void fetchT789Buses()}
          disabled={isLoading}
          className="w-full bg-amber-500 text-slate-900 hover:bg-amber-400"
        >
          {isLoading ? (
            <>
              <LoaderCircle className="animate-spin" />
              Refreshing...
            </>
          ) : (
            <>
              <RefreshCw />
              Refresh T789 Data
            </>
          )}
        </Button>
      }
    >
      {errorMessage ? (
        <section className={panelSectionClass}>
          <p className="inline-flex items-center gap-2 text-sm font-medium text-rose-700">
            <AlertTriangle className="h-4 w-4" />
            T789 Error
          </p>
          <p className="mt-1 text-xs text-slate-600">{errorMessage}</p>
        </section>
      ) : null}

      {etaErrorMessage ? (
        <section className={panelSectionClass}>
          <p className="inline-flex items-center gap-2 text-sm font-medium text-rose-700">
            <AlertTriangle className="h-4 w-4" />
            ETA Error
          </p>
          <p className="mt-1 text-xs text-slate-600">{etaErrorMessage}</p>
        </section>
      ) : null}

      <section className={panelSectionClass}>
        <p className="mb-2 text-sm font-medium text-amber-900">
          All Active T789 Buses ({activeBuses.length})
        </p>
        {!isLoading && !errorMessage && activeBuses.length === 0 ? (
          <p className="text-xs text-slate-600">No active T789 buses right now.</p>
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
                    ? 'border-amber-300 bg-amber-200/20 text-amber-900'
                    : 'hover:bg-amber-100/80 text-slate-800'
                }`}
              >
                <p className="font-medium">
                  Bus {bus.bus_no} · Route {bus.route}
                </p>
                <p className="text-slate-700">
                  {bus.latitude.toFixed(5)}, {bus.longitude.toFixed(5)} ·{' '}
                  {bus.speed.toFixed(1)} km/h
                </p>
                <p className="text-slate-600">
                  Current stop:{' '}
                  {bus.busstop_id
                    ? stopNameById[bus.busstop_id] || bus.busstop_id
                    : 'Unknown'}
                </p>
              </button>
            ))}
          </div>
        ) : null}
      </section>

      <section className={panelSectionClass}>
        <p className="mb-2 text-sm font-medium text-amber-900">
          ETA To KL Gateway ({etas.length})
        </p>
        {!isLoading && !etaErrorMessage && etas.length === 0 ? (
          <p className="text-xs text-slate-600">
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
                    ? 'border-amber-300 bg-amber-200/20 text-amber-900'
                    : 'hover:bg-amber-100/80 text-slate-800'
                }`}
              >
                <p className="font-medium">
                  Bus {eta.bus_no} · Route {eta.route_id || 'T7890'}
                </p>
                <p className="text-slate-700">
                  ETA {eta.eta_minutes.toFixed(1)} min · {eta.stops_away} stops away ·{' '}
                  {eta.distance_km.toFixed(2)} km
                </p>
                <p className="text-slate-600">
                  Current stop:{' '}
                  {stopNameById[eta.current_stop_id] || eta.current_stop_id}
                </p>
              </button>
            ))}
          </div>
        ) : null}
      </section>

      {selectedActiveBus ? (
        <section className={panelSectionClass}>
          <p className="text-sm font-medium text-amber-900">
            Selected Bus {selectedActiveBus.bus_no} · Route {selectedActiveBus.route}
          </p>

          {selectedEta ? (
            <div className="mt-2 rounded-xl border border-amber-200/80 bg-white/72 p-2 text-xs text-slate-800">
              <p className="font-medium">
                ETA to {targetStopName}: {selectedEta.eta_minutes.toFixed(1)} min
              </p>
              <p className="text-slate-600">
                {selectedEta.stops_away} stops away · {selectedEta.distance_km.toFixed(2)} km
              </p>
            </div>
          ) : (
            <p className="mt-2 text-xs text-slate-600">
              No ETA to {targetStopName} for this bus right now.
            </p>
          )}

          {isLoadingRouteShape ? (
            <p className="mt-3 text-xs text-slate-600">Loading route detail...</p>
          ) : routeStops && routeShape ? (
            <div className="mt-3">
              <BusRouteLine
                routeShortName={routeStops.route_short_name}
                routeLongName={routeStops.route_long_name}
                stops={routeStops.stops}
                currentStopId={selectedCurrentStopId}
                currentSequence={selectedCurrentSequence}
                targetStopId={targetStopId}
                targetLabel="KL Gateway target stop"
              />
            </div>
          ) : (
            <p className="mt-3 text-xs text-slate-600">
              Route line is unavailable right now.
            </p>
          )}
        </section>
      ) : null}
    </MapPanelShell>
  )
}
