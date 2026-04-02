import { describe, expect, it } from 'vitest'
import type { RouteShape } from '@/lib/route-geometry'
import {
  buildRoutePrefetchWarning,
  buildVisibleRouteLayers,
  getRouteLineColor,
  getVisibleRouteIds,
  shouldShowRouteStopMarkers,
} from '@/lib/route-map-layers'

describe('getRouteLineColor', () => {
  it('returns deterministic colors for each route id', () => {
    const first = getRouteLineColor('T7890')
    const second = getRouteLineColor('T7890')
    const third = getRouteLineColor('U7800')

    expect(first).toBe(second)
    expect(first).toMatch(/^#[0-9a-f]{6}$/i)
    expect(third).toMatch(/^#[0-9a-f]{6}$/i)
  })
})

describe('getVisibleRouteIds', () => {
  const routeIds = ['T7890', 'U7800', 'T7910']

  it('returns all routes when no route is selected', () => {
    expect(getVisibleRouteIds(routeIds, null)).toEqual(routeIds)
  })

  it('returns only selected route when route is selected', () => {
    expect(getVisibleRouteIds(routeIds, 'U7800')).toEqual(['U7800'])
  })
})

describe('shouldShowRouteStopMarkers', () => {
  it('hides stop markers in all-routes mode and shows for single route', () => {
    expect(shouldShowRouteStopMarkers(null)).toBe(false)
    expect(shouldShowRouteStopMarkers('T7890')).toBe(true)
  })
})

describe('buildVisibleRouteLayers', () => {
  const routeIds = ['T7890', 'U7800']
  const routeStopsByRoute = {
    T7890: {
      stops: [
        {
          stop_id: '1001',
          stop_name: 'Stop A',
          stop_lat: 3.11,
          stop_lon: 101.66,
        },
        {
          stop_id: '1002',
          stop_name: 'Stop B',
          stop_lat: 3.12,
          stop_lon: 101.67,
        },
      ],
    },
    U7800: {
      stops: [
        {
          stop_id: '2001',
          stop_name: 'Stop C',
          stop_lat: 3.13,
          stop_lon: 101.68,
        },
        {
          stop_id: '2002',
          stop_name: 'Stop D',
          stop_lat: 3.14,
          stop_lon: 101.69,
        },
      ],
    },
  }

  const routeShapes: Record<string, RouteShape> = {
    T7890: {
      route_id: 'T7890',
      shape_id: 'shape-t7890',
      points: [
        { lat: 3.115, lon: 101.665, sequence: 1 },
        { lat: 3.116, lon: 101.666, sequence: 2 },
      ],
    },
    U7800: {
      route_id: 'U7800',
      shape_id: 'shape-u7800',
      points: [
        { lat: 3.135, lon: 101.685, sequence: 1 },
        { lat: 3.136, lon: 101.686, sequence: 2 },
      ],
    },
  }

  it('builds overlays for all routes when in all-routes mode', () => {
    const layers = buildVisibleRouteLayers({
      routeIds,
      selectedRouteId: null,
      routeStopsByRoute,
      resolveShape: (routeId) => routeShapes[routeId] ?? null,
    })

    expect(layers).toHaveLength(2)
    expect(layers.map((layer) => layer.id)).toEqual(routeIds)
    layers.forEach((layer) => {
      expect(layer.polylinePoints.length).toBeGreaterThan(1)
      expect(layer.color).toMatch(/^#[0-9a-f]{6}$/i)
    })
  })

  it('builds overlay only for selected route when a tab is chosen', () => {
    const layers = buildVisibleRouteLayers({
      routeIds,
      selectedRouteId: 'U7800',
      routeStopsByRoute,
      resolveShape: (routeId) => routeShapes[routeId] ?? null,
    })

    expect(layers).toHaveLength(1)
    expect(layers[0]?.id).toBe('U7800')
  })
})

describe('buildRoutePrefetchWarning', () => {
  it('returns warning for partial preload failures', () => {
    const warning = buildRoutePrefetchWarning({
      failedRouteIds: ['T7910'],
      totalRouteCount: 3,
    })

    expect(warning).toBe(
      'Loaded 2/3 routes. Some route paths are unavailable.',
    )
  })

  it('deduplicates failed route ids when computing loaded route count', () => {
    const warning = buildRoutePrefetchWarning({
      failedRouteIds: ['T7910', 'T7910'],
      totalRouteCount: 3,
    })

    expect(warning).toBe(
      'Loaded 2/3 routes. Some route paths are unavailable.',
    )
  })

  it('returns null when prefetch is fully successful', () => {
    expect(
      buildRoutePrefetchWarning({
        failedRouteIds: [],
        totalRouteCount: 3,
      }),
    ).toBeNull()
  })
})
