import { buildRoutePolylinePoints, type RouteShape } from '@/lib/route-geometry'

type RouteStopPoint = {
  stop_id: string
  stop_name: string
  stop_lat: number
  stop_lon: number
}

type RouteStopsData = {
  stops: RouteStopPoint[]
}

export type RouteLayer = {
  id: string
  stops: RouteStopPoint[]
  polylinePoints: Array<[number, number]>
  color: string
}

const ROUTE_LINE_PALETTE = [
  '#0ea5e9',
  '#10b981',
  '#f97316',
  '#e11d48',
  '#8b5cf6',
  '#14b8a6',
  '#f59e0b',
  '#ef4444',
  '#6366f1',
  '#22c55e',
]

function hashRouteId(routeId: string): number {
  let hash = 0
  for (let index = 0; index < routeId.length; index += 1) {
    hash = (hash * 31 + routeId.charCodeAt(index)) >>> 0
  }
  return hash
}

export function getRouteLineColor(routeId: string): string {
  const colorIndex = hashRouteId(routeId) % ROUTE_LINE_PALETTE.length
  return ROUTE_LINE_PALETTE[colorIndex] ?? '#06b6d4'
}

export function getVisibleRouteIds(
  routeIds: string[],
  selectedRouteId: string | null,
): string[] {
  if (selectedRouteId === null) {
    return routeIds
  }

  return routeIds.includes(selectedRouteId) ? [selectedRouteId] : []
}

export function shouldShowRouteStopMarkers(
  selectedRouteId: string | null,
): boolean {
  return selectedRouteId !== null
}

export function buildVisibleRouteLayers({
  routeIds,
  selectedRouteId,
  routeStopsByRoute,
  resolveShape,
}: {
  routeIds: string[]
  selectedRouteId: string | null
  routeStopsByRoute: Record<string, RouteStopsData>
  resolveShape: (routeId: string) => RouteShape | null
}): RouteLayer[] {
  const visibleRouteIds = getVisibleRouteIds(routeIds, selectedRouteId)

  return visibleRouteIds
    .map((routeId) => {
      const routeStops = routeStopsByRoute[routeId]
      if (!routeStops) {
        return null
      }

      const polylinePoints = buildRoutePolylinePoints(
        resolveShape(routeId),
        routeStops.stops,
      )
      if (polylinePoints.length < 2 && routeStops.stops.length < 2) {
        return null
      }

      return {
        id: routeId,
        stops: routeStops.stops,
        polylinePoints,
        color: getRouteLineColor(routeId),
      }
    })
    .filter((layer): layer is RouteLayer => layer !== null)
}

export function buildRoutePrefetchWarning({
  failedRouteIds,
  totalRouteCount,
}: {
  failedRouteIds: string[]
  totalRouteCount: number
}): string | null {
  if (failedRouteIds.length === 0 || totalRouteCount <= 0) {
    return null
  }

  const uniqueFailedCount = new Set(failedRouteIds).size
  const loadedCount = Math.max(totalRouteCount - uniqueFailedCount, 0)
  return `Loaded ${loadedCount}/${totalRouteCount} routes. Some route paths are unavailable.`
}