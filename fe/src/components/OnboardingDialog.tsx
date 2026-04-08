import { useState, useEffect, useCallback, useRef } from 'react'
import { createPortal } from 'react-dom'
import { MapPin, Bus, Route, MessageSquarePlus, ChevronRight, ChevronLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'

const STORAGE_KEY = 'rapidbro-onboarding-seen'

const steps = [
  {
    icon: MapPin,
    title: 'Find Your Nearest Stop',
    description:
      'RapidBro uses your location to find the closest bus stop. Allow location access and we\'ll do the rest.',
  },
  {
    icon: Route,
    title: 'Explore Routes',
    description:
      'See all bus routes at your stop. Tap a route to view it on the map, or select "All Routes" to overlay them all.',
  },
  {
    icon: Bus,
    title: 'Track Buses in Real-Time',
    description:
      'View active buses heading to your stop with live ETAs, distance, and how many stops away they are.',
  },
  {
    icon: MessageSquarePlus,
    title: 'Send Us Feedback',
    description:
      'Found a bug or something off? Tap the Feedback button to let us know — we read every message.',
  },
]

function OnboardingDialog() {
  const [open, setOpen] = useState(false)
  const [step, setStep] = useState(0)
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (typeof window === 'undefined') return
    const seen = localStorage.getItem(STORAGE_KEY)
    if (!seen) {
      setOpen(true)
    }
  }, [])

  const handleClose = useCallback(() => {
    setOpen(false)
    localStorage.setItem(STORAGE_KEY, '1')
  }, [])

  useEffect(() => {
    if (!open || typeof window === 'undefined') return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') handleClose()
      if (e.key === 'ArrowRight' && step < steps.length - 1) setStep((s) => s + 1)
      if (e.key === 'ArrowLeft' && step > 0) setStep((s) => s - 1)
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [open, step, handleClose])

  // Trap focus inside the dialog when open
  useEffect(() => {
    if (!open) return
    const el = dialogRef.current
    if (!el) return
    const focusable = el.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    )
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    first?.focus()

    const handleTab = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return
      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault()
          last?.focus()
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault()
          first?.focus()
        }
      }
    }
    el.addEventListener('keydown', handleTab)
    return () => el.removeEventListener('keydown', handleTab)
  }, [open, step])

  if (!open || typeof document === 'undefined') return null

  const current = steps[step]
  const Icon = current.icon
  const isLast = step === steps.length - 1

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      aria-modal="true"
      role="dialog"
      aria-labelledby="onboarding-title"
      aria-describedby="onboarding-description"
      ref={dialogRef}
    >
      <div className="absolute inset-0 bg-slate-900/50 backdrop-blur-sm" aria-hidden="true" />

      <div className="relative z-10 w-full max-w-sm overflow-hidden rounded-3xl border border-amber-200/80 bg-amber-50 shadow-2xl">
        <div className="flex flex-col items-center px-6 pt-8 pb-6 text-center">
          <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-amber-200/60">
            <Icon className="h-8 w-8 text-amber-700" />
          </div>

          <p id="onboarding-title" className="mt-5 font-['Space_Grotesk',_'Avenir_Next',_sans-serif] text-lg font-semibold tracking-tight text-amber-900">
            {current.title}
          </p>
          <p id="onboarding-description" className="mt-2 text-sm leading-relaxed text-slate-600">
            {current.description}
          </p>
        </div>

        {/* Step dots */}
        <div className="flex justify-center gap-1.5 pb-5">
          {steps.map((_, i) => (
            <button
              key={i}
              type="button"
              onClick={() => setStep(i)}
              aria-label={`Go to step ${i + 1}`}
              className={`h-2 rounded-full transition-all ${
                i === step ? 'w-6 bg-amber-500' : 'w-2 bg-amber-300/60'
              }`}
            />
          ))}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-between border-t border-amber-200/80 px-5 py-4">
          {step > 0 ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setStep((s) => s - 1)}
              className="text-slate-600"
            >
              <ChevronLeft className="h-4 w-4" />
              Back
            </Button>
          ) : (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleClose}
              className="text-slate-500"
            >
              Skip
            </Button>
          )}

          {isLast ? (
            <Button
              type="button"
              onClick={handleClose}
              size="sm"
              className="bg-amber-500 text-slate-900 hover:bg-amber-400"
            >
              Get started
            </Button>
          ) : (
            <Button
              type="button"
              onClick={() => setStep((s) => s + 1)}
              size="sm"
              className="bg-amber-500 text-slate-900 hover:bg-amber-400"
            >
              Next
              <ChevronRight className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>
    </div>,
    document.body,
  )
}

export { OnboardingDialog }
