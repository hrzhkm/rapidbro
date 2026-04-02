import { useState } from 'react'
import { PanelBottomOpen, PanelBottomClose } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { getMobilePanelTransformClass } from '@/lib/map-layout'

type MapPanelShellProps = {
  map: React.ReactNode
  panelTitle: string
  panelDescription?: string
  panelStatus?: React.ReactNode
  panelActions?: React.ReactNode
  children: React.ReactNode
}

function MapPanelShell({
  map,
  panelTitle,
  panelDescription,
  panelStatus,
  panelActions,
  children,
}: MapPanelShellProps) {
  const [isMobilePanelOpen, setIsMobilePanelOpen] = useState(true)

  return (
    <main className="relative h-[calc(100dvh-3.5rem)] overflow-hidden bg-slate-950">
      <div className="absolute inset-0 z-0">{map}</div>

      <div className="pointer-events-none absolute inset-0 z-10 bg-[radial-gradient(circle_at_14%_18%,rgba(251,191,36,0.18),transparent_42%),radial-gradient(circle_at_86%_76%,rgba(34,211,238,0.16),transparent_38%),linear-gradient(to_bottom,rgba(15,23,42,0.26),rgba(15,23,42,0.1)_28%,rgba(15,23,42,0.22))]" />

      <aside
        data-testid="map-side-panel"
        data-mobile-open={isMobilePanelOpen}
        className={cn(
          'absolute inset-x-0 bottom-0 z-30 max-h-[84dvh] rounded-t-3xl border border-white/25 bg-slate-900/78 text-slate-100 shadow-[0_-20px_55px_rgba(2,6,23,0.5)] backdrop-blur-xl transition-transform duration-300 md:inset-y-4 md:right-4 md:left-auto md:w-[26.5rem] md:max-h-none md:rounded-3xl md:translate-y-0',
          getMobilePanelTransformClass(isMobilePanelOpen),
        )}
      >
        <div className="flex items-start justify-between gap-3 border-b border-white/15 px-4 py-3.5 md:px-5">
          <div className="min-w-0">
            <p className="font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-base font-semibold tracking-tight text-amber-100">
              {panelTitle}
            </p>
            {panelDescription ? (
              <p className="mt-0.5 text-xs text-slate-300">{panelDescription}</p>
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
          <div className="border-b border-white/10 px-4 py-3 md:px-5">{panelActions}</div>
        ) : null}

        <div className="max-h-[calc(84dvh-8.6rem)] space-y-4 overflow-y-auto px-4 py-4 md:max-h-[calc(100dvh-12.5rem)] md:px-5 md:pb-5">
          {children}
        </div>
      </aside>
    </main>
  )
}

export { MapPanelShell }
