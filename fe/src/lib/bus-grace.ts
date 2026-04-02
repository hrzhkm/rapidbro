export type BusGraceMergeArgs<T> = {
  incoming: T[]
  previous: T[]
  fetchSucceeded: boolean
  nowMs: number
  graceWindowMs: number
  lastNonEmptyAtMs: number | null
}

export type BusGraceMergeResult<T> = {
  buses: T[]
  isGraceHeld: boolean
  lastNonEmptyAtMs: number | null
}

export function mergeBusesWithGraceHold<T>({
  incoming,
  previous,
  fetchSucceeded,
  nowMs,
  graceWindowMs,
  lastNonEmptyAtMs,
}: BusGraceMergeArgs<T>): BusGraceMergeResult<T> {
  if (incoming.length > 0) {
    return {
      buses: incoming,
      isGraceHeld: false,
      lastNonEmptyAtMs: nowMs,
    }
  }

  const canHoldPrevious =
    previous.length > 0 &&
    lastNonEmptyAtMs !== null &&
    nowMs - lastNonEmptyAtMs <= graceWindowMs

  if (canHoldPrevious) {
    return {
      buses: previous,
      isGraceHeld: true,
      lastNonEmptyAtMs,
    }
  }

  if (!fetchSucceeded) {
    return {
      buses: previous,
      isGraceHeld: false,
      lastNonEmptyAtMs,
    }
  }

  return {
    buses: [],
    isGraceHeld: false,
    lastNonEmptyAtMs,
  }
}
