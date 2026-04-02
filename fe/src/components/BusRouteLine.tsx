type RouteLineStop = {
  stop_id: string
  stop_name: string
  sequence: number
}

type BusRouteLineProps = {
  routeShortName?: string
  routeLongName?: string
  stops: RouteLineStop[]
  currentStopId: string | null
  currentSequence?: number | null
  targetStopId?: string | null
  targetLabel?: string
  currentLabel?: string
}

function BusRouteLine({
  routeShortName,
  routeLongName,
  stops,
  currentStopId,
  currentSequence,
  targetStopId = null,
  targetLabel = 'Target stop',
  currentLabel = 'Bus is here now',
}: BusRouteLineProps) {
  const resolvedCurrentSequence =
    currentSequence ??
    stops.find((stop) => stop.stop_id === currentStopId)?.sequence ??
    null
  const targetSequence =
    stops.find((stop) => stop.stop_id === targetStopId)?.sequence ?? null

  return (
    <div className="rounded-md border bg-amber-50/40 p-3">
      {routeShortName || routeLongName ? (
        <div className="mb-4 rounded-md border bg-background p-3">
          {routeShortName ? (
            <p className="text-sm font-medium">{routeShortName}</p>
          ) : null}
          {routeLongName ? (
            <p className="text-sm text-muted-foreground">{routeLongName}</p>
          ) : null}
        </div>
      ) : null}

      <div className="space-y-0">
        {stops.map((stop, index) => {
          const isCurrentStop = currentStopId === stop.stop_id
          const isTargetStop = targetStopId === stop.stop_id
          const isPassed =
            resolvedCurrentSequence !== null &&
            stop.sequence < resolvedCurrentSequence
          const isBetweenCurrentAndTarget =
            resolvedCurrentSequence !== null &&
            targetSequence !== null &&
            stop.sequence > resolvedCurrentSequence &&
            stop.sequence < targetSequence
          const isUpcoming =
            !isBetweenCurrentAndTarget &&
            !isTargetStop &&
            resolvedCurrentSequence !== null &&
            stop.sequence > resolvedCurrentSequence

          const markerClassName = isCurrentStop
            ? 'border-amber-900 bg-amber-500 ring-amber-200'
            : isTargetStop
              ? 'border-orange-800 bg-orange-500 ring-orange-200'
              : isPassed
                ? 'border-lime-800 bg-lime-500 ring-lime-100'
                : isBetweenCurrentAndTarget
                  ? 'border-lime-900 bg-lime-400 ring-lime-200'
                  : isUpcoming
                    ? 'border-amber-400 bg-amber-200 ring-amber-100'
                    : 'border-border bg-background ring-background'
          const lineInnerClassName = isPassed
            ? 'bg-lime-500'
            : isBetweenCurrentAndTarget
              ? 'bg-amber-300'
              : isUpcoming
                ? 'bg-amber-200'
                : 'bg-border'

          return (
            <div
              key={stop.stop_id}
              className="grid grid-cols-[1.5rem_1fr] gap-3"
            >
              <div className="flex flex-col items-center">
                <div
                  className={`h-4 w-4 rounded-full border-[3px] shadow-sm ring-2 ${markerClassName}`}
                />
                {index < stops.length - 1 ? (
                  <div className="relative min-h-8 w-3 flex-1">
                    <div className="absolute inset-y-0 left-1/2 w-[6px] -translate-x-1/2 rounded-full bg-amber-700/70" />
                    <div
                      className={`absolute inset-y-[1px] left-1/2 w-[3px] -translate-x-1/2 rounded-full ${lineInnerClassName}`}
                    />
                  </div>
                ) : null}
              </div>

              <div
                className={`pb-5 ${
                  isCurrentStop || isTargetStop
                    ? 'text-foreground'
                    : 'text-muted-foreground'
                }`}
              >
                <p className="text-sm font-medium">{stop.stop_name}</p>
                <p className="text-xs">Stop ID: {stop.stop_id}</p>
                {isCurrentStop ? (
                  <p className="mt-1 text-xs font-medium">{currentLabel}</p>
                ) : null}
                {isTargetStop ? (
                  <p className="mt-1 text-xs font-medium">{targetLabel}</p>
                ) : null}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

export { BusRouteLine }
