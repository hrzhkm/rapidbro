import { describe, expect, it } from 'vitest'
import { detectRegion } from '@/lib/region'
import { MOCK_ALOR_SETAR_COORDS, MOCK_KANGAR_COORDS } from '@/lib/geolocation'

describe('detectRegion', () => {
  it('returns empty string for KL / default area', () => {
    expect(detectRegion(3.149, 101.698)).toBe('')
  })

  it('returns /alor-setar for Alor Setar city center', () => {
    expect(detectRegion(6.1177, 100.3621)).toBe('/alor-setar')
  })

  it('returns /kangar for Kangar city center', () => {
    expect(detectRegion(6.4414, 100.1986)).toBe('/kangar')
  })

  it('returns /kangar at lat 6.35 (kangar latMin matches first in array)', () => {
    expect(detectRegion(6.35, 100.4)).toBe('/kangar')
  })

  it('returns /alor-setar just below lat 6.35', () => {
    expect(detectRegion(6.34, 100.4)).toBe('/alor-setar')
  })

  it('returns empty string for Singapore (outside all regions)', () => {
    expect(detectRegion(1.3521, 103.8198)).toBe('')
  })

  it('returns empty string for coordinates outside all defined region boxes', () => {
    expect(detectRegion(4.0, 103.0)).toBe('')
  })

  it('MOCK_KANGAR_COORDS resolves to /kangar region', () => {
    expect(detectRegion(MOCK_KANGAR_COORDS.latitude, MOCK_KANGAR_COORDS.longitude)).toBe('/kangar')
  })

  it('MOCK_ALOR_SETAR_COORDS resolves to /alor-setar region', () => {
    expect(
      detectRegion(MOCK_ALOR_SETAR_COORDS.latitude, MOCK_ALOR_SETAR_COORDS.longitude),
    ).toBe('/alor-setar')
  })
})
