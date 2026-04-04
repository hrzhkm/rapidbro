import { useState } from 'react'
import { PanelBottomOpen, PanelBottomClose } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { getMobilePanelTransformClass } from '@/lib/map-layout'

type MapPanelShellProps = {
  map: React.ReactNode
  mapOverlay?: React.ReactNode
  panelTitle: string
  panelDescription?: string
  panelStatus?: React.ReactNode
  panelActions?: React.ReactNode
  children: React.ReactNode
}

function MapPanelShell({
  map,
  mapOverlay,
  panelTitle,
  panelDescription,
  panelStatus,
  panelActions,
  children,
}: MapPanelShellProps) {
  const [isMobilePanelOpen, setIsMobilePanelOpen] = useState(false)

  return (
    <main className="relative h-[calc(100dvh-3.5rem)] overflow-hidden bg-amber-50">
      <div className="absolute inset-0 z-0">{map}</div>

      <div className="pointer-events-none absolute inset-0 z-10 bg-[radial-gradient(circle_at_14%_18%,rgba(251,191,36,0.22),transparent_44%),radial-gradient(circle_at_86%_76%,rgba(34,211,238,0.18),transparent_40%),linear-gradient(to_bottom,rgba(255,251,235,0.12),rgba(255,251,235,0.02)_30%,rgba(251,191,36,0.12))]" />

      {mapOverlay ? (
        <div className="absolute inset-0 z-20 flex items-center justify-center p-6 md:pr-[28rem]">
          {mapOverlay}
        </div>
      ) : null}

      <aside
        data-testid="map-side-panel"
        data-mobile-open={isMobilePanelOpen}
        className={cn(
          'absolute inset-x-0 bottom-2 z-30 flex max-h-[88dvh] flex-col rounded-t-3xl border border-amber-200/85 bg-amber-50/84 text-slate-800 shadow-[0_-20px_55px_rgba(148,163,184,0.28)] backdrop-blur-xl transition-transform duration-300 md:inset-y-4 md:right-4 md:left-auto md:w-[26.5rem] md:max-h-none md:rounded-3xl md:translate-y-0',
          getMobilePanelTransformClass(isMobilePanelOpen),
        )}
        style={{
          paddingBottom: 'max(env(safe-area-inset-bottom), 0.5rem)',
        }}
      >
        <div className="flex items-start justify-between gap-3 border-b border-amber-200/85 px-4 py-3.5 md:px-5">
          <div className="min-w-0">
            <p className="font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-base font-semibold tracking-tight text-amber-900">
              {panelTitle}
            </p>
            {panelDescription ? (
              <p className="mt-0.5 text-xs text-slate-600">{panelDescription}</p>
            ) : null}
            {panelStatus ? <div className="mt-2 text-xs">{panelStatus}</div> : null}
          </div>

          <Button
            type="button"
            variant="outline"
            size="sm"
            className="md:hidden"
            onClick={() => setIsMobilePanelOpen((current) => !current)}
            aria-label={isMobilePanelOpen ? 'Collapse panel' : 'Expand panel'}
          >
            {isMobilePanelOpen ? <PanelBottomClose /> : <PanelBottomOpen />}
            {isMobilePanelOpen ? 'Collapse' : 'Expand'}
          </Button>
        </div>

        {panelActions ? (
          <div className="border-b border-amber-200/70 px-4 py-3 md:px-5">{panelActions}</div>
        ) : null}

        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-4 md:px-5 md:pb-5">
          {children}
        </div>
      </aside>
    </main>
  )
}

export { MapPanelShell }
