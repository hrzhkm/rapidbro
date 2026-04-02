import { Link, useRouterState } from '@tanstack/react-router'
import { isMapSurfaceRoute } from '@/lib/map-layout'
import { cn } from '@/lib/utils'

export default function Header() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  })
  const isMapPage = isMapSurfaceRoute(pathname)

  return (
    <header
      className={cn(
        'fixed inset-x-0 top-0 z-40 border-b',
        isMapPage
          ? 'border-white/20 bg-slate-950/45 text-white backdrop-blur-md'
          : 'border-border bg-background text-foreground',
      )}
    >
      <div
        className={cn(
          'mx-auto flex h-14 items-center justify-between gap-4 px-4',
          isMapPage ? 'max-w-none' : 'max-w-4xl',
        )}
      >
        <div className="flex items-center gap-4">
          <Link
            to="/"
            className={cn(
              "font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-sm font-semibold tracking-tight",
              isMapPage ? 'text-amber-100' : 'text-foreground',
            )}
          >
            RapidBro
          </Link>
          <Link
            to="/t789"
            className={cn(
              'text-sm transition-colors',
              isMapPage
                ? 'text-slate-200 hover:text-amber-100'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            T789 ETA
          </Link>
        </div>
        <p
          className={cn(
            'text-xs',
            isMapPage ? 'text-slate-200' : 'text-muted-foreground',
          )}
        >
          Nearest bus stop lookup
        </p>
      </div>
    </header>
  )
}
