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
  try {
    return await requestPosition(geolocation, COARSE_GEOLOCATION_OPTIONS)
  } catch {
    return requestPosition(geolocation, PRECISE_GEOLOCATION_OPTIONS)
  }
}

export { COARSE_GEOLOCATION_OPTIONS, PRECISE_GEOLOCATION_OPTIONS }
