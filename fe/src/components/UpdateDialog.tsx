import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { Sparkles, MapPin, MessageSquarePlus, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/dialog'
import { submitFeedback } from '@/serverFunctions/feedback'
import { Send, LoaderCircle, CheckCircle2 } from 'lucide-react'

const UPDATE_KEY = 'rapidbro-update-seen-v2'

const BAS_MY_CITIES = [
  'Kangar',
  'Alor Setar',
  'Kota Bharu',
  'Kuala Terengganu',
  'Ipoh',
  'Seremban',
  'Melaka',
  'Johor Bahru',
  'Kuching',
]

function FeedbackInline({ onClose }: { onClose: () => void }) {
  const [message, setMessage] = useState('')
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle')
  const [errorText, setErrorText] = useState('')

  const handleSubmit = async () => {
    const trimmed = message.trim()
    if (!trimmed) {
      setErrorText('Please write something before submitting.')
      return
    }
    setStatus('loading')
    setErrorText('')
    try {
      // @ts-expect-error -- data is typed as undefined without .validator() but works at runtime
      await submitFeedback({ data: { message: trimmed } })
      setStatus('success')
    } catch {
      setStatus('error')
      setErrorText('Something went wrong. Please try again.')
    }
  }

  if (status === 'success') {
    return (
      <div className="flex flex-col items-center gap-2 py-4 text-center">
        <CheckCircle2 className="h-8 w-8 text-emerald-500" />
        <p className="text-sm font-medium text-slate-800">Thanks for your feedback!</p>
        <p className="text-xs text-slate-500">We'll look into it and make things better.</p>
        <Button
          type="button"
          size="sm"
          onClick={onClose}
          className="mt-2 bg-amber-500 text-slate-900 hover:bg-amber-400"
        >
          Done
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <textarea
        value={message}
        onChange={(e) => {
          setMessage(e.target.value)
          if (errorText) setErrorText('')
        }}
        placeholder="Tell us what you think, or report a bug..."
        rows={3}
        maxLength={1000}
        disabled={status === 'loading'}
        className="w-full resize-none rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-800 placeholder:text-slate-400 focus:border-amber-400 focus:outline-none focus:ring-2 focus:ring-amber-200 disabled:opacity-50"
      />
      {errorText && <p className="text-xs text-rose-600">{errorText}</p>}
      <div className="flex gap-2">
        <Button
          type="button"
          size="sm"
          variant="ghost"
          onClick={onClose}
          className="flex-1 text-slate-500"
        >
          Skip
        </Button>
        <Button
          type="button"
          size="sm"
          onClick={handleSubmit}
          disabled={status === 'loading' || !message.trim()}
          className="flex-1 bg-amber-500 text-slate-900 hover:bg-amber-400 disabled:opacity-50"
        >
          {status === 'loading' ? (
            <>
              <LoaderCircle className="animate-spin" />
              Sending...
            </>
          ) : (
            <>
              <Send className="h-3.5 w-3.5" />
              Send
            </>
          )}
        </Button>
      </div>
    </div>
  )
}

type View = 'update' | 'feedback'

function UpdateDialog() {
  const [open, setOpen] = useState(false)
  const [view, setView] = useState<View>('update')

  useEffect(() => {
    if (typeof window === 'undefined') return
    const seen = localStorage.getItem(UPDATE_KEY)
    if (!seen) setOpen(true)
  }, [])

  const handleClose = useCallback(() => {
    setOpen(false)
    localStorage.setItem(UPDATE_KEY, '1')
  }, [])

  useEffect(() => {
    if (!open || typeof window === 'undefined') return
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') handleClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [open, handleClose])

  if (!open || typeof document === 'undefined') return null

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-slate-900/50 backdrop-blur-sm"
        onClick={handleClose}
      />

      <div className="relative z-10 w-full max-w-sm overflow-hidden rounded-3xl border border-amber-200/80 bg-white shadow-2xl">
        {/* Header */}
        <div className="relative bg-gradient-to-br from-amber-400 to-amber-500 px-6 pt-6 pb-5">
          <button
            type="button"
            onClick={handleClose}
            aria-label="Close"
            className="absolute right-4 top-4 flex h-7 w-7 items-center justify-center rounded-full bg-amber-600/30 text-amber-900 transition-colors hover:bg-amber-600/50"
          >
            <X className="h-4 w-4" />
          </button>

          <div className="flex items-center gap-2.5">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-amber-600/30">
              <Sparkles className="h-5 w-5 text-amber-900" />
            </div>
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-widest text-amber-800/80">
                What&apos;s new
              </p>
              <p className="font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-base font-bold leading-tight text-amber-950">
                RapidBro Update
              </p>
            </div>
          </div>
        </div>

        {view === 'update' ? (
          <div className="px-6 py-5 space-y-5">
            {/* BAS.MY support */}
            <div>
              <div className="flex items-center gap-2 mb-3">
                <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-amber-100">
                  <MapPin className="h-4 w-4 text-amber-600" />
                </div>
                <p className="text-sm font-semibold text-slate-800">BAS.MY Support</p>
              </div>
              <p className="text-xs text-slate-500 mb-2.5 leading-relaxed">
                We now support BAS.MY routes across 9 cities in Peninsular Malaysia and Sarawak.
              </p>
              <div className="flex flex-wrap gap-1.5">
                {BAS_MY_CITIES.map((city) => (
                  <span
                    key={city}
                    className="rounded-full border border-amber-200 bg-amber-50 px-2.5 py-0.5 text-xs font-medium text-amber-800"
                  >
                    {city}
                  </span>
                ))}
              </div>
            </div>

            <div className="border-t border-slate-100" />

            {/* UI refresh */}
            <div className="flex items-start gap-2.5">
              <div className="mt-0.5 flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-lg bg-slate-100">
                <Sparkles className="h-4 w-4 text-slate-500" />
              </div>
              <div>
                <p className="text-sm font-semibold text-slate-800">Updated UI</p>
                <p className="mt-0.5 text-xs text-slate-500 leading-relaxed">
                  Cleaner layout, smoother interactions, and a refreshed look across the whole app.
                </p>
              </div>
            </div>

            <div className="border-t border-slate-100" />

            {/* Feedback prompt */}
            <div className="flex items-start gap-2.5">
              <div className="mt-0.5 flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-lg bg-slate-100">
                <MessageSquarePlus className="h-4 w-4 text-slate-500" />
              </div>
              <div>
                <p className="text-sm font-semibold text-slate-800">Leave Feedback</p>
                <p className="mt-0.5 text-xs text-slate-500 leading-relaxed">
                  Something not working? We read every message.
                </p>
              </div>
            </div>

            {/* Actions */}
            <div className="flex gap-2 pt-1">
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => setView('feedback')}
                className="flex-1 border-slate-200 text-slate-600 hover:border-amber-300 hover:bg-amber-50 hover:text-amber-800"
              >
                <MessageSquarePlus className="h-3.5 w-3.5" />
                Feedback
              </Button>
              <Button
                type="button"
                size="sm"
                onClick={handleClose}
                className="flex-1 bg-amber-500 text-slate-900 hover:bg-amber-400"
              >
                Got it
              </Button>
            </div>
          </div>
        ) : (
          <div className="px-6 py-5 space-y-3">
            <button
              type="button"
              onClick={() => setView('update')}
              className="flex items-center gap-1 text-xs text-amber-600 hover:text-amber-700"
            >
              ← Back
            </button>
            <p className="text-sm font-semibold text-slate-800">Send Feedback</p>
            <p className="text-xs text-slate-500">
              Found a bug or something off? Let us know — we read every message.
            </p>
            <FeedbackInline onClose={handleClose} />
          </div>
        )}
      </div>
    </div>,
    document.body,
  )
}

export { UpdateDialog }
