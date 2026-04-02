import { describe, expect, it } from 'vitest'
import { buildRoutePolylinePoints } from '@/lib/route-geometry'

describe('buildRoutePolylinePoints', () => {
  it('uses GTFS shape points when shape data is available', () => {
    const polyline = buildRoutePolylinePoints(
      {
        route_id: 'U7800',
        shape_id: 'U780001',
        points: [
          { lat: 3.1144, lon: 101.6651, sequence: 1 },
          { lat: 3.1152, lon: 101.6669, sequence: 2 },
          { lat: 3.1164, lon: 101.6692, sequence: 3 },
        ],
      },
      [
        { stop_lat: 3.11, stop_lon: 101.66 },
        { stop_lat: 3.12, stop_lon: 101.67 },
      ],
    )

    expect(polyline).toEqual([
      [3.1144, 101.6651],
      [3.1152, 101.6669],
      [3.1164, 101.6692],
    ])
  })

  it('falls back to stop-to-stop points if shape data is missing', () => {
    const polyline = buildRoutePolylinePoints(null, [
      { stop_lat: 3.1101, stop_lon: 101.6605 },
      { stop_lat: 3.1118, stop_lon: 101.6637 },
      { stop_lat: 3.1139, stop_lon: 101.6661 },
    ])

    expect(polyline).toEqual([
      [3.1101, 101.6605],
      [3.1118, 101.6637],
      [3.1139, 101.6661],
    ])
  })

  it('returns empty polyline when there are not enough points', () => {
    expect(
      buildRoutePolylinePoints(
        {
          route_id: 'T7890',
          shape_id: 'T789002',
          points: [{ lat: 3.12, lon: 101.67, sequence: 1 }],
        },
        [{ stop_lat: 3.11, stop_lon: 101.66 }],
      ),
    ).toEqual([])
  })
})
