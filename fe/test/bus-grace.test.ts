import { describe, expect, it } from 'vitest'
import { mergeBusesWithGraceHold } from '@/lib/bus-grace'

describe('mergeBusesWithGraceHold', () => {
  it('stores fresh buses and marks state as non-stale', () => {
    const nowMs = 1_000
    const result = mergeBusesWithGraceHold({
      incoming: [{ id: 'A' }],
      previous: [{ id: 'OLD' }],
      fetchSucceeded: true,
      nowMs,
      graceWindowMs: 300_000,
      lastNonEmptyAtMs: null,
    })

    expect(result.buses).toEqual([{ id: 'A' }])
    expect(result.isGraceHeld).toBe(false)
    expect(result.lastNonEmptyAtMs).toBe(nowMs)
  })

  it('keeps previous buses during a temporary empty response within grace window', () => {
    const result = mergeBusesWithGraceHold({
      incoming: [],
      previous: [{ id: 'A' }],
      fetchSucceeded: true,
      nowMs: 200_000,
      graceWindowMs: 300_000,
      lastNonEmptyAtMs: 50_000,
    })

    expect(result.buses).toEqual([{ id: 'A' }])
    expect(result.isGraceHeld).toBe(true)
    expect(result.lastNonEmptyAtMs).toBe(50_000)
  })

  it('drops buses after grace window expiry on successful empty response', () => {
    const result = mergeBusesWithGraceHold({
      incoming: [],
      previous: [{ id: 'A' }],
      fetchSucceeded: true,
      nowMs: 400_001,
      graceWindowMs: 300_000,
      lastNonEmptyAtMs: 100_000,
    })

    expect(result.buses).toEqual([])
    expect(result.isGraceHeld).toBe(false)
    expect(result.lastNonEmptyAtMs).toBe(100_000)
  })

  it('keeps previous buses on fetch failure without forcing stale badge when no valid hold window', () => {
    const result = mergeBusesWithGraceHold({
      incoming: [],
      previous: [{ id: 'A' }],
      fetchSucceeded: false,
      nowMs: 400_001,
      graceWindowMs: 300_000,
      lastNonEmptyAtMs: 100_000,
    })

    expect(result.buses).toEqual([{ id: 'A' }])
    expect(result.isGraceHeld).toBe(false)
    expect(result.lastNonEmptyAtMs).toBe(100_000)
  })
})
