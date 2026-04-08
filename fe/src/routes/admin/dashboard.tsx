import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import { deleteFeedback, getFeedback } from '@/serverFunctions/admin'

type Feedback = {
  id: number
  message: string
  region: string | null
  createdAt: Date | string
}

export const Route = createFileRoute('/admin/dashboard')({
  component: RouteComponent,
})

function StatCard({
  label,
  value,
  icon,
  color = 'amber',
}: {
  label: string
  value: number
  icon: React.ReactNode
  color?: 'amber' | 'slate' | 'emerald' | 'rose'
}) {
  const colors = {
    amber: 'bg-amber-50 border-amber-100 text-amber-600',
    slate: 'bg-slate-50 border-slate-200 text-slate-500',
    emerald: 'bg-emerald-50 border-emerald-100 text-emerald-600',
    rose: 'bg-rose-50 border-rose-100 text-rose-600',
  }
  return (
    <div className="bg-white rounded-2xl border border-slate-200 p-4 flex items-center gap-4 shadow-sm">
      <div className={`rounded-xl border p-2.5 ${colors[color]}`}>{icon}</div>
      <div>
        <p className="text-xs font-medium text-slate-500 uppercase tracking-wide">{label}</p>
        <p className="text-2xl font-semibold text-slate-800">{value}</p>
      </div>
    </div>
  )
}

function RegionBadge({ region }: { region: string | null }) {
  if (!region) return <span className="text-slate-400 text-xs">—</span>
  return (
    <span className="inline-flex items-center rounded-full bg-amber-100 border border-amber-200 px-2 py-0.5 text-xs font-medium text-amber-700">
      {region}
    </span>
  )
}

function formatDate(date: Date | string) {
  const d = new Date(date)
  return d.toLocaleString('en-MY', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function isToday(date: Date | string) {
  const d = new Date(date)
  const now = new Date()
  return d.toDateString() === now.toDateString()
}

function isThisWeek(date: Date | string) {
  const d = new Date(date)
  const now = new Date()
  const weekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000)
  return d >= weekAgo
}

export default function RouteComponent() {
  const navigate = useNavigate()
  const [feedback, setFeedback] = useState<Feedback[]>([])
  const [loading, setLoading] = useState(true)
  const [deletingId, setDeletingId] = useState<number | null>(null)
  const [search, setSearch] = useState('')
  const [regionFilter, setRegionFilter] = useState<string>('all')

  useEffect(() => {
    const auth = sessionStorage.getItem('admin_authenticated')
    if (auth !== 'true') {
      navigate({ to: '/admin/login' })
      return
    }
    loadFeedback()
  }, [])

  async function loadFeedback() {
    setLoading(true)
    try {
      const data = await getFeedback()
      setFeedback(data as Feedback[])
    } finally {
      setLoading(false)
    }
  }

  async function handleDelete(id: number) {
    setDeletingId(id)
    try {
      // @ts-expect-error -- data typed as undefined without .validator()
      await deleteFeedback({ data: { id } })
      setFeedback((prev) => prev.filter((f) => f.id !== id))
    } finally {
      setDeletingId(null)
    }
  }

  function handleLogout() {
    sessionStorage.removeItem('admin_authenticated')
    navigate({ to: '/admin/login' })
  }

  const regions = ['all', ...Array.from(new Set(feedback.map((f) => f.region).filter(Boolean) as string[]))]

  const filtered = feedback.filter((f) => {
    const matchSearch = f.message.toLowerCase().includes(search.toLowerCase())
    const matchRegion = regionFilter === 'all' || f.region === regionFilter
    return matchSearch && matchRegion
  })

  const todayCount = feedback.filter((f) => isToday(f.createdAt)).length
  const weekCount = feedback.filter((f) => isThisWeek(f.createdAt)).length
  const regionCount = new Set(feedback.map((f) => f.region).filter(Boolean)).size

  return (
    <div className="min-h-screen bg-slate-50">
      {/* Header */}
      <header className="bg-white border-b border-slate-200 sticky top-0 z-10">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="inline-flex items-center justify-center w-8 h-8 rounded-lg bg-amber-100 border border-amber-200">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="text-amber-600">
                <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                <polyline points="9 22 9 12 15 12 15 22" />
              </svg>
            </div>
            <div>
              <span className="text-sm font-semibold text-slate-800">RapidBro</span>
              <span className="ml-1.5 text-xs text-slate-400">Admin</span>
            </div>
          </div>
          <button
            onClick={handleLogout}
            className="flex items-center gap-1.5 text-xs font-medium text-slate-500 hover:text-slate-800 transition px-2 py-1.5 rounded-lg hover:bg-slate-100 cursor-pointer"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
              <polyline points="16 17 21 12 16 7" />
              <line x1="21" y1="12" x2="9" y2="12" />
            </svg>
            Logout
          </button>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-4 sm:px-6 py-8 space-y-6">
        {/* Page title */}
        <div>
          <h1 className="text-xl font-semibold text-slate-800">Feedback</h1>
          <p className="text-sm text-slate-500 mt-0.5">User-submitted feedback from the app</p>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <StatCard
            label="Total"
            value={feedback.length}
            color="amber"
            icon={
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
              </svg>
            }
          />
          <StatCard
            label="Today"
            value={todayCount}
            color="emerald"
            icon={
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <polyline points="12 6 12 12 16 14" />
              </svg>
            }
          />
          <StatCard
            label="This week"
            value={weekCount}
            color="slate"
            icon={
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
                <line x1="16" y1="2" x2="16" y2="6" />
                <line x1="8" y1="2" x2="8" y2="6" />
                <line x1="3" y1="10" x2="21" y2="10" />
              </svg>
            }
          />
          <StatCard
            label="Regions"
            value={regionCount}
            color="rose"
            icon={
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 22s-8-4.5-8-11.8A8 8 0 0 1 12 2a8 8 0 0 1 8 8.2c0 7.3-8 11.8-8 11.8z" />
                <circle cx="12" cy="10" r="3" />
              </svg>
            }
          />
        </div>

        {/* Filters & table */}
        <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden">
          {/* Toolbar */}
          <div className="p-4 border-b border-slate-100 flex flex-col sm:flex-row gap-3">
            <div className="relative flex-1">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                type="text"
                placeholder="Search feedback…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="w-full rounded-lg border border-slate-200 bg-slate-50 pl-8 pr-3 py-2 text-sm text-slate-800 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-transparent transition"
              />
            </div>
            <select
              value={regionFilter}
              onChange={(e) => setRegionFilter(e.target.value)}
              className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-700 focus:outline-none focus:ring-2 focus:ring-amber-400 focus:border-transparent transition cursor-pointer"
            >
              {regions.map((r) => (
                <option key={r} value={r}>
                  {r === 'all' ? 'All regions' : r}
                </option>
              ))}
            </select>
            <button
              onClick={loadFeedback}
              className="flex items-center gap-1.5 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-600 hover:bg-slate-100 transition cursor-pointer"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
              Refresh
            </button>
          </div>

          {/* Table */}
          {loading ? (
            <div className="flex items-center justify-center py-16 text-slate-400">
              <svg className="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              Loading…
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-16 text-slate-400">
              <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="mb-3 opacity-50">
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
              </svg>
              <p className="text-sm">No feedback found</p>
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {filtered.map((item) => (
                <div key={item.id} className="px-4 py-3.5 flex items-start gap-4 hover:bg-slate-50 transition group">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-slate-700 leading-relaxed wrap-break-word">{item.message}</p>
                    <div className="flex items-center gap-2 mt-1.5">
                      <RegionBadge region={item.region} />
                      <span className="text-xs text-slate-400">{formatDate(item.createdAt)}</span>
                    </div>
                  </div>
                  <button
                    onClick={() => handleDelete(item.id)}
                    disabled={deletingId === item.id}
                    className="opacity-0 group-hover:opacity-100 transition shrink-0 p-1.5 rounded-lg text-slate-400 hover:text-rose-500 hover:bg-rose-50 disabled:opacity-50 cursor-pointer"
                    aria-label="Delete feedback"
                  >
                    {deletingId === item.id ? (
                      <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                      </svg>
                    ) : (
                      <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="3 6 5 6 21 6" />
                        <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                        <path d="M10 11v6" />
                        <path d="M14 11v6" />
                        <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
                      </svg>
                    )}
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Footer */}
          {!loading && filtered.length > 0 && (
            <div className="px-4 py-3 border-t border-slate-100 text-xs text-slate-400">
              Showing {filtered.length} of {feedback.length} entries
            </div>
          )}
        </div>
      </main>
    </div>
  )
}
