export type ApiRegionPrefix =
  | ''
  | '/kangar'
  | '/alor-setar'
  | '/kota-bharu'
  | '/kuala-terengganu'
  | '/ipoh'
  | '/seremban-a'
  | '/seremban-b'
  | '/melaka'
  | '/johor'
  | '/kuching'

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
  {
    prefix: '/kota-bharu',
    name: 'Kota Bharu',
    latMin: 4.8,
    latMax: 6.3,
    lonMin: 101.8,
    lonMax: 102.65,
  },
  {
    prefix: '/kuala-terengganu',
    name: 'Kuala Terengganu',
    latMin: 5.0,
    latMax: 5.6,
    lonMin: 102.9,
    lonMax: 103.25,
  },
  {
    prefix: '/ipoh',
    name: 'Ipoh',
    latMin: 3.85,
    latMax: 4.9,
    lonMin: 100.9,
    lonMax: 101.35,
  },
  {
    prefix: '/seremban-a',
    name: 'Seremban',
    latMin: 2.4,
    latMax: 2.9,
    lonMin: 101.65,
    lonMax: 102.25,
  },
  {
    prefix: '/seremban-b',
    name: 'Seremban',
    latMin: 2.35,
    latMax: 3.15,
    lonMin: 101.7,
    lonMax: 102.45,
  },
  {
    prefix: '/melaka',
    name: 'Melaka',
    latMin: 2.15,
    latMax: 2.5,
    lonMin: 102.05,
    lonMax: 102.6,
  },
  {
    prefix: '/johor',
    name: 'Johor Bahru',
    latMin: 1.3,
    latMax: 1.75,
    lonMin: 103.35,
    lonMax: 104.0,
  },
  {
    prefix: '/kuching',
    name: 'Kuching',
    latMin: 1.1,
    latMax: 1.7,
    lonMin: 110.1,
    lonMax: 110.6,
  },
]

export function detectRegion(lat: number, lon: number): ApiRegionPrefix {
  const match = REGIONS.find(
    (r) => lat >= r.latMin && lat <= r.latMax && lon >= r.lonMin && lon <= r.lonMax,
  )
  return match?.prefix ?? ''
}

export function getRegionName(prefix: ApiRegionPrefix): string {
  if (prefix === '') return 'Klang Valley'
  return REGIONS.find((r) => r.prefix === prefix)?.name ?? prefix.replace('/', '')
}
