'use client'

import { useEffect, useRef, useState } from 'react'
import { cn } from '@/lib/utils'

type RouteMapStop = {
  stop_id: string
  stop_name: string
  stop_lat: number
  stop_lon: number
}

type BusRouteMapProps = {
  stops: RouteMapStop[]
  polylinePoints?: Array<[number, number]>
  routeLayers?: Array<{
    id: string
    stops: RouteMapStop[]
    polylinePoints?: Array<[number, number]>
    color?: string
  }>
  buses?: Array<{
    id: string
    label: string
    lat: number
    lon: number
    isSelected?: boolean
  }>
  trackedStop?: RouteMapStop | null
  currentStopId?: string | null
  targetStopId?: string | null
  fullScreen?: boolean
  showLegend?: boolean
  showStopMarkers?: boolean
  className?: string
}

type LeafletModule = typeof import('leaflet')

type LeafletState = {
  leaflet: LeafletModule
  map: import('leaflet').Map
  layerGroup: import('leaflet').LayerGroup
}

type PolylineCompatibleStop = {
  stop_lat: number
  stop_lon: number
}

type RoutePoint = [number, number]

type RouteDirectionArrow = {
  lat: number
  lon: number
  bearing: number
}

type RenderableRouteLayer = {
  stops: RouteMapStop[]
  polylinePoints: Array<[number, number]>
}

function getFitBoundsOptions(fullScreen: boolean) {
  const basePadding: [number, number] = [24, 24]
  if (typeof window === 'undefined') {
    return { padding: basePadding }
  }

  const isDesktopViewport = window.matchMedia('(min-width: 768px)').matches
  if (fullScreen && isDesktopViewport) {
    // Keep route geometry centered in the visible map area when the side panel overlays the map.
    return {
      paddingTopLeft: basePadding,
      paddingBottomRight: [460, 24] as [number, number],
    }
  }

  return { padding: basePadding }
}

export function resolvePolylinePointsForRendering(
  stops: PolylineCompatibleStop[],
  polylinePoints: Array<[number, number]>,
): Array<[number, number]> {
  if (polylinePoints.length > 1) {
    return polylinePoints
  }

  return stops.map((stop) => [stop.stop_lat, stop.stop_lon] as [number, number])
}

function getDistanceMeters(a: RoutePoint, b: RoutePoint): number {
  const [lat1, lon1] = a
  const [lat2, lon2] = b
  const toRadians = (degrees: number) => (degrees * Math.PI) / 180
  const earthRadiusMeters = 6_371_000
  const deltaLat = toRadians(lat2 - lat1)
  const deltaLon = toRadians(lon2 - lon1)
  const startLat = toRadians(lat1)
  const endLat = toRadians(lat2)

  const haversine =
    Math.sin(deltaLat / 2) * Math.sin(deltaLat / 2) +
    Math.cos(startLat) *
      Math.cos(endLat) *
      Math.sin(deltaLon / 2) *
      Math.sin(deltaLon / 2)
  const centralAngle = 2 * Math.atan2(Math.sqrt(haversine), Math.sqrt(1 - haversine))

  return earthRadiusMeters * centralAngle
}

function getBearingDegrees(a: RoutePoint, b: RoutePoint): number {
  const [lat1, lon1] = a
  const [lat2, lon2] = b
  const toRadians = (degrees: number) => (degrees * Math.PI) / 180
  const toDegrees = (radians: number) => (radians * 180) / Math.PI

  const phi1 = toRadians(lat1)
  const phi2 = toRadians(lat2)
  const lambdaDelta = toRadians(lon2 - lon1)
  const y = Math.sin(lambdaDelta) * Math.cos(phi2)
  const x =
    Math.cos(phi1) * Math.sin(phi2) -
    Math.sin(phi1) * Math.cos(phi2) * Math.cos(lambdaDelta)
  const theta = toDegrees(Math.atan2(y, x))

  return (theta + 360) % 360
}

export function buildRouteDirectionArrows(
  polylinePoints: RoutePoint[],
  spacingMeters = 380,
  maxArrows = 24,
): RouteDirectionArrow[] {
  if (polylinePoints.length < 2 || spacingMeters <= 0 || maxArrows <= 0) {
    return []
  }

  const segments = polylinePoints
    .slice(1)
    .map((to, index) => {
      const from = polylinePoints[index]
      const lengthMeters = getDistanceMeters(from, to)
      return {
        from,
        to,
        lengthMeters,
        bearing: getBearingDegrees(from, to),
      }
    })
    .filter((segment) => segment.lengthMeters > 0)

  if (segments.length === 0) {
    return []
  }

  const totalLengthMeters = segments.reduce(
    (sum, segment) => sum + segment.lengthMeters,
    0,
  )
  if (totalLengthMeters <= 0) {
    return []
  }

  const requestedArrowCount = Math.floor(totalLengthMeters / spacingMeters)
  const arrowCount = Math.min(Math.max(requestedArrowCount, 1), maxArrows)
  const intervalMeters = totalLengthMeters / (arrowCount + 1)
  const arrows: RouteDirectionArrow[] = []

  for (let index = 1; index <= arrowCount; index += 1) {
    const targetDistance = intervalMeters * index
    let traversedMeters = 0

    for (const segment of segments) {
      if (traversedMeters + segment.lengthMeters < targetDistance) {
        traversedMeters += segment.lengthMeters
        continue
      }

      const distanceInSegment = targetDistance - traversedMeters
      const ratio = Math.min(
        Math.max(distanceInSegment / segment.lengthMeters, 0),
        1,
      )
      const lat = segment.from[0] + (segment.to[0] - segment.from[0]) * ratio
      const lon = segment.from[1] + (segment.to[1] - segment.from[1]) * ratio

      arrows.push({
        lat,
        lon,
        bearing: segment.bearing,
      })
      break
    }
  }

  return arrows
}

export function getRouteArrowRenderConfig(routeCount: number): {
  spacingMeters: number
  maxArrows: number
} {
  if (routeCount <= 1) {
    return {
      spacingMeters: 380,
      maxArrows: 24,
    }
  }

  // In all-routes overlays, reduce arrow density so route flow remains visible without clutter.
  return {
    spacingMeters: 1100,
    maxArrows: 4,
  }
}

export function collectBoundsPointsForRendering({
  routeLayers,
  showStopMarkers,
  buses,
  trackedStop,
}: {
  routeLayers: RenderableRouteLayer[]
  showStopMarkers: boolean
  buses: Array<{ lat: number; lon: number }>
  trackedStop: RouteMapStop | null
}): RoutePoint[] {
  const routePoints = routeLayers.flatMap((routeLayer) => routeLayer.polylinePoints)
  const stopPoints = showStopMarkers
    ? routeLayers.flatMap((routeLayer) =>
        routeLayer.stops.map((stop) => [stop.stop_lat, stop.stop_lon] as RoutePoint),
      )
    : []
  const busPoints = buses.map((bus) => [bus.lat, bus.lon] as RoutePoint)
  const trackedStopPoints = trackedStop
    ? [[trackedStop.stop_lat, trackedStop.stop_lon] as RoutePoint]
    : []

  return [...routePoints, ...stopPoints, ...busPoints, ...trackedStopPoints]
}

async function loadLeaflet() {
  return import('leaflet')
}

export function getBusMarkerScale(zoom: number): number {
  // Keep markers readable at low zoom while avoiding oversized overlap.
  if (zoom <= 13) return 0.72
  if (zoom <= 14) return 0.82
  if (zoom <= 15) return 0.92
  if (zoom <= 16) return 1
  return 1.08
}

function BusRouteMap({
  stops,
  polylinePoints = [],
  routeLayers = [],
  buses = [],
  trackedStop = null,
  currentStopId = null,
  targetStopId = null,
  fullScreen = false,
  showLegend = true,
  showStopMarkers = true,
  className,
}: BusRouteMapProps) {
  const mapContainerRef = useRef<HTMLDivElement | null>(null)
  const leafletStateRef = useRef<LeafletState | null>(null)
  const hasFitBoundsRef = useRef(false)
  const [mapReadyTick, setMapReadyTick] = useState(0)

  useEffect(() => {
    hasFitBoundsRef.current = false
  }, [stops, polylinePoints, routeLayers, showStopMarkers])

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

      const applyBusMarkerScale = () => {
        const scale = getBusMarkerScale(map.getZoom()).toString()
        if (!mapContainerRef.current) {
          return
        }

        const busElements =
          mapContainerRef.current.querySelectorAll<HTMLElement>('.bus-live-marker')
        busElements.forEach((element) => {
          element.style.setProperty('--bus-marker-scale', scale)
        })
      }
      map.on('zoomend', applyBusMarkerScale)

      const layerGroup = leaflet.layerGroup().addTo(map)
      leafletStateRef.current = { leaflet, map, layerGroup }

      requestAnimationFrame(() => {
        map.invalidateSize()
        applyBusMarkerScale()
        // Ensure route layers are rendered once the async Leaflet map is ready.
        setMapReadyTick((current) => current + 1)
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
      const resolvedRouteLayers =
        routeLayers.length > 0
          ? routeLayers
              .map((layer) => ({
                id: layer.id,
                color: layer.color ?? '#06b6d4',
                stops: layer.stops,
                polylinePoints: resolvePolylinePointsForRendering(
                  layer.stops,
                  layer.polylinePoints ?? [],
                ),
              }))
              .filter((layer) => layer.polylinePoints.length > 1)
          : [
              {
                id: 'default',
                color: '#06b6d4',
                stops,
                polylinePoints: resolvePolylinePointsForRendering(
                  stops,
                  polylinePoints,
                ),
              },
            ].filter((layer) => layer.polylinePoints.length > 1)

      if (resolvedRouteLayers.length === 0) {
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
      const lineLatLngsByRoute = resolvedRouteLayers.map((layer) =>
        layer.polylinePoints.map(([lat, lon]) => leaflet.latLng(lat, lon)),
      )

      resolvedRouteLayers.forEach((routeLayer, routeIndex) => {
        const lineLatLngs = lineLatLngsByRoute[routeIndex]
        if (!lineLatLngs) {
          return
        }

        leaflet
          .polyline(lineLatLngs, {
            color: '#0f172a',
            weight: resolvedRouteLayers.length > 1 ? 7 : 8,
            opacity: 0.75,
            lineCap: 'round',
            lineJoin: 'round',
          })
          .addTo(layerGroup)

        leaflet
          .polyline(lineLatLngs, {
            color: routeLayer.color,
            weight: resolvedRouteLayers.length > 1 ? 4 : 4.5,
            opacity: 0.96,
            lineCap: 'round',
            lineJoin: 'round',
          })
          .addTo(layerGroup)
      })

      const arrowConfig = getRouteArrowRenderConfig(resolvedRouteLayers.length)
      resolvedRouteLayers.forEach((routeLayer) => {
        const directionArrows = buildRouteDirectionArrows(
          routeLayer.polylinePoints,
          arrowConfig.spacingMeters,
          arrowConfig.maxArrows,
        )
        directionArrows.forEach((arrow) => {
          leaflet
            .marker([arrow.lat, arrow.lon], {
              icon: leaflet.divIcon({
                className: 'route-direction-arrow-icon',
                html: `<div class="route-direction-arrow" style="--route-arrow-rotation:${arrow.bearing}deg"></div>`,
                iconSize: [18, 18],
                iconAnchor: [9, 9],
              }),
              keyboard: false,
              interactive: false,
            })
            .addTo(layerGroup)
        })
      })

      if (showStopMarkers) {
        const seenStopIds = new Set<string>()
        resolvedRouteLayers.forEach((routeLayer) => {
          routeLayer.stops.forEach((stop) => {
            if (seenStopIds.has(stop.stop_id)) {
              return
            }
            seenStopIds.add(stop.stop_id)

            const isCurrent = stop.stop_id === currentStopId
            const isTarget = stop.stop_id === targetStopId

            const pinFill = isCurrent
              ? '#f59e0b'
              : isTarget
                ? '#f97316'
                : '#facc15'
            const pinStroke = isCurrent || isTarget ? '#78350f' : '#92400e'
            const pinSize = isCurrent || isTarget ? 32 : 24
            const pinAnchorY = pinSize
            const pinSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="${pinSize}" height="${pinSize}" viewBox="0 0 24 24" fill="${pinFill}" stroke="${pinStroke}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 10c0 4.993-7.942 10.751-7.942 10.751a.1.1 0 0 1-.117 0S4 14.993 4 10a8 8 0 0 1 16 0"/><circle cx="12" cy="10" r="3" fill="${pinStroke}" stroke="none"/></svg>`

            leaflet
              .marker([stop.stop_lat, stop.stop_lon], {
                icon: leaflet.divIcon({
                  className: '',
                  html: pinSvg,
                  iconSize: [pinSize, pinSize],
                  iconAnchor: [pinSize / 2, pinAnchorY],
                }),
              })
              .bindTooltip(stop.stop_name)
              .addTo(layerGroup)
          })
        })
      }

      if (trackedStop) {
        const trackedPinSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 24 24" fill="#f97316" stroke="#7c2d12" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 10c0 4.993-7.942 10.751-7.942 10.751a.1.1 0 0 1-.117 0S4 14.993 4 10a8 8 0 0 1 16 0"/><circle cx="12" cy="10" r="3" fill="#7c2d12" stroke="none"/></svg>`
        leaflet
          .marker([trackedStop.stop_lat, trackedStop.stop_lon], {
            icon: leaflet.divIcon({
              className: '',
              html: trackedPinSvg,
              iconSize: [36, 36],
              iconAnchor: [18, 36],
            }),
          })
          .bindTooltip(`${trackedStop.stop_name} (tracked stop)`)
          .addTo(layerGroup)
      }

      buses.forEach((bus) => {
        const markerScale = getBusMarkerScale(map.getZoom())
        leaflet
          .marker([bus.lat, bus.lon], {
            icon: leaflet.divIcon({
              className: 'bus-live-marker-icon',
              html: `<div class="bus-live-marker${bus.isSelected ? ' is-selected' : ''}" style="--bus-marker-scale:${markerScale}"><img src="/bus-icon.png" alt="" class="bus-live-marker-image" /></div>`,
              iconSize: [44, 44],
              iconAnchor: [22, 22],
            }),
          })
          .bindTooltip(bus.label)
          .addTo(layerGroup)
      })

      const boundsLatLngs = collectBoundsPointsForRendering({
        routeLayers: resolvedRouteLayers,
        showStopMarkers,
        buses,
        trackedStop,
      }).map(([lat, lon]) => leaflet.latLng(lat, lon))
      if (!hasFitBoundsRef.current) {
        map.fitBounds(leaflet.latLngBounds(boundsLatLngs), {
          ...getFitBoundsOptions(fullScreen),
          animate: false,
        })
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
  }, [
    stops,
    polylinePoints,
    routeLayers,
    buses,
    currentStopId,
    targetStopId,
    fullScreen,
    showStopMarkers,
    trackedStop,
    mapReadyTick,
  ])

  const hasRenderableSingleRoute = stops.length >= 2 || polylinePoints.length >= 2
  const hasRenderableRouteLayers =
    routeLayers.some(
      (layer) =>
        (layer.polylinePoints?.length ?? 0) >= 2 || layer.stops.length >= 2,
    )

  if (!hasRenderableSingleRoute && !hasRenderableRouteLayers) {
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
        className={cn(
          'bus-route-map rounded-md border',
          fullScreen ? 'is-fullscreen rounded-none border-0' : null,
        )}
        aria-label="Bus route map"
      />
      {showLegend ? (
        <p className="mt-2 text-xs text-muted-foreground">
          Current bus and target stop are emphasized on the route.
        </p>
      ) : null}
    </div>
  )
}

export { BusRouteMap }
