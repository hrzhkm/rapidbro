import { describe, expect, it } from 'vitest'
import {
  buildRouteDirectionArrows,
  getRouteArrowRenderConfig,
  resolvePolylinePointsForRendering,
} from '@/components/BusRouteMap'

describe('resolvePolylinePointsForRendering', () => {
  it('uses explicit polyline points when available', () => {
    expect(
      resolvePolylinePointsForRendering(
        [
          { stop_lat: 3.11, stop_lon: 101.66 },
          { stop_lat: 3.12, stop_lon: 101.67 },
        ],
        [
          [3.1144, 101.6651],
          [3.1152, 101.6669],
        ],
      ),
    ).toEqual([
      [3.1144, 101.6651],
      [3.1152, 101.6669],
    ])
  })

  it('falls back to stop points when explicit polyline is incomplete', () => {
    expect(
      resolvePolylinePointsForRendering(
        [
          { stop_lat: 3.1111, stop_lon: 101.6611 },
          { stop_lat: 3.1122, stop_lon: 101.6622 },
          { stop_lat: 3.1133, stop_lon: 101.6633 },
        ],
        [[3.2, 101.7]],
      ),
    ).toEqual([
      [3.1111, 101.6611],
      [3.1122, 101.6622],
      [3.1133, 101.6633],
    ])
  })
})

describe('buildRouteDirectionArrows', () => {
  it('returns no arrows when route has fewer than 2 points', () => {
    expect(buildRouteDirectionArrows([[3.11, 101.66]])).toEqual([])
  })

  it('builds arrows along the route with consistent spacing', () => {
    const route: Array<[number, number]> = [
      [3.11, 101.66],
      [3.12, 101.66],
      [3.13, 101.66],
    ]
    const arrows = buildRouteDirectionArrows(route, 500, 10)

    expect(arrows.length).toBeGreaterThan(1)
    expect(arrows.length).toBeLessThanOrEqual(10)
    arrows.forEach((arrow) => {
      expect(arrow.lon).toBeCloseTo(101.66, 5)
      expect(arrow.bearing).toBeGreaterThanOrEqual(0)
      expect(arrow.bearing).toBeLessThan(360)
    })
    expect(arrows[0]?.lat).toBeGreaterThan(route[0][0])
    expect(arrows[arrows.length - 1]?.lat).toBeLessThan(route[2][0])
  })

  it('limits arrow count with maxArrows', () => {
    const route: Array<[number, number]> = [
      [3.0, 101.0],
      [3.2, 101.0],
    ]
    const arrows = buildRouteDirectionArrows(route, 100, 3)
    expect(arrows).toHaveLength(3)
  })

  it('uses near-0 degree bearing for northbound segment', () => {
    const route: Array<[number, number]> = [
      [3.1, 101.66],
      [3.2, 101.66],
    ]
    const arrows = buildRouteDirectionArrows(route, 1500, 1)
    expect(arrows).toHaveLength(1)
    const bearing = arrows[0]?.bearing ?? -1
    expect(bearing < 0.5 || bearing > 359.5).toBe(true)
  })

  it('uses near-90 degree bearing for eastbound segment', () => {
    const route: Array<[number, number]> = [
      [3.1, 101.66],
      [3.1, 101.76],
    ]
    const arrows = buildRouteDirectionArrows(route, 1500, 1)
    expect(arrows).toHaveLength(1)
    expect(arrows[0]?.bearing).toBeGreaterThan(89.5)
    expect(arrows[0]?.bearing).toBeLessThan(90.5)
  })
})

describe('getRouteArrowRenderConfig', () => {
  it('uses dense arrows for single-route maps', () => {
    expect(getRouteArrowRenderConfig(1)).toEqual({
      spacingMeters: 380,
      maxArrows: 24,
    })
  })

  it('uses sparse arrows for multi-route overlays', () => {
    expect(getRouteArrowRenderConfig(3)).toEqual({
      spacingMeters: 1100,
      maxArrows: 4,
    })
  })
})
