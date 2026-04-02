// @vitest-environment jsdom

import { render, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { BusRouteMap } from '@/components/BusRouteMap'

const addToMock = vi.fn().mockReturnThis()
const bindTooltipMock = vi.fn().mockReturnThis()
const clearLayersMock = vi.fn()
const mapOnMock = vi.fn()
const mapSetViewMock = vi.fn()
const mapFitBoundsMock = vi.fn()
const mapInvalidateSizeMock = vi.fn()
const mapRemoveMock = vi.fn()
const mapGetZoomMock = vi.fn(() => 14)
const polylineMock = vi.fn(() => ({ addTo: addToMock }))

vi.mock('leaflet', () => ({
  map: vi.fn(() => ({
    setView: mapSetViewMock,
    on: mapOnMock,
    getZoom: mapGetZoomMock,
    fitBounds: mapFitBoundsMock,
    invalidateSize: mapInvalidateSizeMock,
    remove: mapRemoveMock,
  })),
  tileLayer: vi.fn(() => ({ addTo: addToMock })),
  layerGroup: vi.fn(() => ({
    addTo: addToMock,
    clearLayers: clearLayersMock,
  })),
  polyline: polylineMock,
  circleMarker: vi.fn(() => ({
    bindTooltip: bindTooltipMock,
    addTo: addToMock,
  })),
  marker: vi.fn(() => ({
    bindTooltip: bindTooltipMock,
    addTo: addToMock,
  })),
  divIcon: vi.fn(() => ({})),
  latLng: vi.fn((lat: number, lon: number) => ({ lat, lng: lon })),
}))

describe('BusRouteMap', () => {
  it('renders route polylines on initial load after async map setup', async () => {
    render(
      <BusRouteMap
        stops={[
          {
            stop_id: 'A',
            stop_name: 'Stop A',
            stop_lat: 3.1144,
            stop_lon: 101.6651,
          },
          {
            stop_id: 'B',
            stop_name: 'Stop B',
            stop_lat: 3.1152,
            stop_lon: 101.6669,
          },
        ]}
        polylinePoints={[
          [3.1144, 101.6651],
          [3.1152, 101.6669],
        ]}
      />,
    )

    await waitFor(() => {
      // Dark casing + bright core.
      expect(polylineMock).toHaveBeenCalledTimes(2)
    })
  })
})
