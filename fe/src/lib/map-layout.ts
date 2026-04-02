export function isMapSurfaceRoute(pathname: string): boolean {
  return pathname === '/' || pathname === '/t789'
}

export function getMobilePanelTransformClass(isPanelOpen: boolean): string {
  return isPanelOpen
    ? 'translate-y-0'
    : 'translate-y-[calc(100%-3.5rem)] md:translate-y-0'
}
