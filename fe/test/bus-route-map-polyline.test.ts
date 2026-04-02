import { describe, expect, it } from 'vitest'
import { resolvePolylinePointsForRendering } from '@/components/BusRouteMap'

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
