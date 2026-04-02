import { describe, expect, it } from 'vitest'
import { getBusMarkerScale } from '@/components/BusRouteMap'

describe('getBusMarkerScale', () => {
  it('scales down markers at low zoom and up at high zoom', () => {
    expect(getBusMarkerScale(13)).toBe(0.72)
    expect(getBusMarkerScale(14)).toBe(0.82)
    expect(getBusMarkerScale(15)).toBe(0.92)
    expect(getBusMarkerScale(16)).toBe(1)
    expect(getBusMarkerScale(18)).toBe(1.08)
  })
})
