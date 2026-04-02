import { describe, expect, it } from 'vitest'
import {
  getMobilePanelTransformClass,
  isMapSurfaceRoute,
} from '@/lib/map-layout'

describe('map layout helpers', () => {
  it('identifies routes that should use map-surface chrome', () => {
    expect(isMapSurfaceRoute('/')).toBe(true)
    expect(isMapSurfaceRoute('/t789')).toBe(true)
    expect(isMapSurfaceRoute('/test')).toBe(false)
  })

  it('returns the correct mobile transform class', () => {
    expect(getMobilePanelTransformClass(true)).toBe('translate-y-0')
    expect(getMobilePanelTransformClass(false)).toBe(
      'translate-y-[calc(100%-4.5rem)] md:translate-y-0',
    )
  })
})
