export type ApiRegionPrefix = '' | '/kangar' | '/alor-setar'

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
    latMin: 6.35,
    latMax: 6.75,
    lonMin: 100.0,
    lonMax: 100.55,
  },
  {
    prefix: '/alor-setar',
    name: 'Alor Setar',
    latMin: 5.5,
    latMax: 6.35,
    lonMin: 100.2,
    lonMax: 100.65,
  },
]

export function detectRegion(lat: number, lon: number): ApiRegionPrefix {
  const match = REGIONS.find(
    (r) => lat >= r.latMin && lat <= r.latMax && lon >= r.lonMin && lon <= r.lonMax,
  )
  return match?.prefix ?? ''
}
