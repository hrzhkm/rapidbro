const COARSE_GEOLOCATION_OPTIONS: PositionOptions = {
  enableHighAccuracy: false,
  timeout: 3500,
  maximumAge: 120000,
}

const PRECISE_GEOLOCATION_OPTIONS: PositionOptions = {
  enableHighAccuracy: true,
  timeout: 10000,
  maximumAge: 30000,
}

// ── Mock coordinates (dev only) ───────────────────────────────────────────────
// Set VITE_MOCK_KANGAR=true in .env.local to simulate a Kangar-area user.
// Set VITE_MOCK_ALOR_SETAR=true in .env.local to simulate an Alor Setar-area user.

const MOCK_KANGAR_COORDS = {
  latitude: 6.4414,
  longitude: 100.1986,
  accuracy: 150,
} as const

const MOCK_ALOR_SETAR_COORDS = {
  latitude: 6.1177,
  longitude: 100.3621,
  accuracy: 150,
} as const

function makeMockPosition(coords: {
  latitude: number
  longitude: number
  accuracy: number
}): GeolocationPosition {
  return {
    coords: {
      latitude: coords.latitude,
      longitude: coords.longitude,
      accuracy: coords.accuracy,
      altitude: null,
      altitudeAccuracy: null,
      heading: null,
      speed: null,
    },
    timestamp: Date.now(),
  } as GeolocationPosition
}

function requestPosition(
  geolocation: Pick<Geolocation, 'getCurrentPosition'>,
  options: PositionOptions,
): Promise<GeolocationPosition> {
  return new Promise((resolve, reject) => {
    geolocation.getCurrentPosition(resolve, reject, options)
  })
}

export async function getGeolocationPosition(
  geolocation: Pick<Geolocation, 'getCurrentPosition'>,
): Promise<GeolocationPosition> {
  if (import.meta.env.VITE_MOCK_ALOR_SETAR === 'true') {
    return makeMockPosition(MOCK_ALOR_SETAR_COORDS)
  }
  if (import.meta.env.VITE_MOCK_KANGAR === 'true') {
    return makeMockPosition(MOCK_KANGAR_COORDS)
  }

  try {
    return await requestPosition(geolocation, COARSE_GEOLOCATION_OPTIONS)
  } catch {
    return requestPosition(geolocation, PRECISE_GEOLOCATION_OPTIONS)
  }
}

export {
  COARSE_GEOLOCATION_OPTIONS,
  PRECISE_GEOLOCATION_OPTIONS,
  MOCK_KANGAR_COORDS,
  MOCK_ALOR_SETAR_COORDS,
}
