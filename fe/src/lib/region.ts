export type ApiRegionPrefix = '' | '/kangar'

type RegionDefinition = {
  prefix: ApiRegionPrefix
  name: string
  latMin: number
  latMax: number
  lonMin: number
  lonMax: number
}

const REGIONS: RegionDefinition[] = [
  {
    prefix: '/kangar',
    name: 'Kangar',
    latMin: 5.5,
    latMax: 6.7,
    lonMin: 99.5,
    lonMax: 101.0,
  },
]

export function detectRegion(lat: number, lon: number): ApiRegionPrefix {
  const match = REGIONS.find(
    (r) => lat >= r.latMin && lat <= r.latMax && lon >= r.lonMin && lon <= r.lonMax,
  )
  return match?.prefix ?? ''
}
