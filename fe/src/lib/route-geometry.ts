export type RouteShapePoint = {
  lat: number
  lon: number
  sequence: number
}

export type RouteShape = {
  route_id: string
  shape_id: string
  points: RouteShapePoint[]
}

export type RouteStopPoint = {
  stop_lat: number
  stop_lon: number
}

export function buildRoutePolylinePoints(
  shape: RouteShape | null | undefined,
  stops: RouteStopPoint[] | null | undefined,
): Array<[number, number]> {
  if (shape && shape.points.length > 1) {
    return shape.points.map((point) => [point.lat, point.lon])
  }

  if (stops && stops.length > 1) {
    return stops.map((stop) => [stop.stop_lat, stop.stop_lon])
  }

  return []
}
