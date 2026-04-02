import { useEffect, useRef } from 'react'

type RouteMapStop = {
  stop_id: string
  stop_name: string
  stop_lat: number
  stop_lon: number
}

type BusRouteMapProps = {
  stops: RouteMapStop[]
  polylinePoints?: Array<[number, number]>
  currentStopId?: string | null
  targetStopId?: string | null
  className?: string
}

type LeafletModule = typeof import('leaflet')

type LeafletState = {
  leaflet: LeafletModule
  map: import('leaflet').Map
  layerGroup: import('leaflet').LayerGroup
}

async function loadLeaflet() {
  return import('leaflet')
}

function BusRouteMap({
  stops,
  polylinePoints = [],
  currentStopId = null,
  targetStopId = null,
  className,
}: BusRouteMapProps) {
  const mapContainerRef = useRef<HTMLDivElement | null>(null)
  const leafletStateRef = useRef<LeafletState | null>(null)
  const hasFitBoundsRef = useRef(false)

  useEffect(() => {
    hasFitBoundsRef.current = false
  }, [stops, polylinePoints])

  useEffect(() => {
    let disposed = false

    const setupMap = async () => {
      if (typeof window === 'undefined' || !mapContainerRef.current) {
        return
      }
      if (leafletStateRef.current) {
        return
      }

      const leaflet = await loadLeaflet()
      if (disposed || !mapContainerRef.current) {
        return
      }

      const map = leaflet.map(mapContainerRef.current, {
        zoomControl: true,
        attributionControl: true,
      })
      const initialCenter =
        stops[0] !== undefined
          ? [stops[0].stop_lat, stops[0].stop_lon]
          : [3.139, 101.6869]
      map.setView(initialCenter as [number, number], 14)

      leaflet
        .tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
          maxZoom: 19,
          attribution:
            '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>',
        })
        .addTo(map)

      const layerGroup = leaflet.layerGroup().addTo(map)
      leafletStateRef.current = { leaflet, map, layerGroup }

      requestAnimationFrame(() => {
        map.invalidateSize()
      })
    }

    setupMap()

    return () => {
      disposed = true
      const state = leafletStateRef.current
      state?.layerGroup.clearLayers()
      state?.map.remove()
      leafletStateRef.current = null
    }
  }, [])

  useEffect(() => {
    let disposed = false

    const renderLayers = async () => {
      const fallbackStopPolylinePoints = stops.map(
        (stop) => [stop.stop_lat, stop.stop_lon] as [number, number],
      )
      const resolvedPolylinePoints =
        polylinePoints.length > 1 ? polylinePoints : fallbackStopPolylinePoints
      if (resolvedPolylinePoints.length < 2) {
        return
      }

      const state = leafletStateRef.current
      if (!state) {
        return
      }

      const { leaflet, map, layerGroup } = state
      if (disposed) {
        return
      }

      layerGroup.clearLayers()
      const lineLatLngs = resolvedPolylinePoints.map(([lat, lon]) =>
        leaflet.latLng(lat, lon),
      )

      // High-contrast route line: dark casing + bright core.
      leaflet
        .polyline(lineLatLngs, {
          color: '#0f172a',
          weight: 8,
          opacity: 0.85,
          lineCap: 'round',
          lineJoin: 'round',
        })
        .addTo(layerGroup)

      leaflet
        .polyline(lineLatLngs, {
          color: '#06b6d4',
          weight: 4,
          opacity: 0.95,
          lineCap: 'round',
          lineJoin: 'round',
        })
        .addTo(layerGroup)

      stops.forEach((stop) => {
        const isCurrent = stop.stop_id === currentStopId
        const isTarget = stop.stop_id === targetStopId

        leaflet
          .circleMarker([stop.stop_lat, stop.stop_lon], {
            radius: isCurrent || isTarget ? 8 : 4,
            color: isCurrent || isTarget ? '#78350f' : '#92400e',
            weight: isCurrent || isTarget ? 3 : 2,
            fillColor: isCurrent ? '#f59e0b' : isTarget ? '#f97316' : '#facc15',
            fillOpacity: 0.95,
          })
          .bindTooltip(stop.stop_name)
          .addTo(layerGroup)
      })

      const boundsLatLngs =
        stops.length > 1
          ? [
              ...lineLatLngs,
              ...stops.map((stop) => leaflet.latLng(stop.stop_lat, stop.stop_lon)),
            ]
          : lineLatLngs
      if (!hasFitBoundsRef.current) {
        map.fitBounds(boundsLatLngs, { padding: [24, 24], animate: false })
        hasFitBoundsRef.current = true
      }

      requestAnimationFrame(() => {
        map.invalidateSize()
      })
    }

    renderLayers()

    return () => {
      disposed = true
    }
  }, [stops, polylinePoints, currentStopId, targetStopId])

  if (stops.length < 2 && polylinePoints.length < 2) {
    return (
      <div className="rounded-md border bg-muted/20 p-3 text-sm text-muted-foreground">
        Route map is unavailable because there are not enough points.
      </div>
    )
  }

  return (
    <div className={className} data-testid="bus-route-map">
      <div
        ref={mapContainerRef}
        className="bus-route-map rounded-md border"
        aria-label="Bus route map"
      />
      <p className="mt-2 text-xs text-muted-foreground">
        Current bus and target stop are emphasized on the route.
      </p>
    </div>
  )
}

export { BusRouteMap }
