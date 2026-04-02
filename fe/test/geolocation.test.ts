import { describe, expect, it, vi } from 'vitest'
import {
  COARSE_GEOLOCATION_OPTIONS,
  getGeolocationPosition,
  PRECISE_GEOLOCATION_OPTIONS,
} from '@/lib/geolocation'

function mockPosition(): GeolocationPosition {
  return {
    coords: {
      latitude: 3.111,
      longitude: 101.66,
      accuracy: 18,
      altitude: null,
      altitudeAccuracy: null,
      heading: null,
      speed: null,
      toJSON: () => ({}),
    },
    timestamp: Date.now(),
    toJSON: () => ({}),
  }
}

describe('getGeolocationPosition', () => {
  it('uses coarse geolocation options first', async () => {
    const position = mockPosition()
    const getCurrentPosition = vi.fn((success: PositionCallback) => {
      success(position)
    })

    const result = await getGeolocationPosition({ getCurrentPosition })

    expect(result).toBe(position)
    expect(getCurrentPosition).toHaveBeenCalledTimes(1)
    expect(getCurrentPosition).toHaveBeenCalledWith(
      expect.any(Function),
      expect.any(Function),
      COARSE_GEOLOCATION_OPTIONS,
    )
  })

  it('falls back to precise options after coarse failure', async () => {
    const position = mockPosition()
    const getCurrentPosition = vi.fn(
      (success: PositionCallback, error: PositionErrorCallback, options?: PositionOptions) => {
        if (options?.enableHighAccuracy) {
          success(position)
          return
        }
        error({
          code: 3,
          message: 'timeout',
          PERMISSION_DENIED: 1,
          POSITION_UNAVAILABLE: 2,
          TIMEOUT: 3,
        } as GeolocationPositionError)
      },
    )

    const result = await getGeolocationPosition({ getCurrentPosition })

    expect(result).toBe(position)
    expect(getCurrentPosition).toHaveBeenNthCalledWith(
      1,
      expect.any(Function),
      expect.any(Function),
      COARSE_GEOLOCATION_OPTIONS,
    )
    expect(getCurrentPosition).toHaveBeenNthCalledWith(
      2,
      expect.any(Function),
      expect.any(Function),
      PRECISE_GEOLOCATION_OPTIONS,
    )
  })
})
