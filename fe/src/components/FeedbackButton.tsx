import { useState } from 'react'
import { MessageSquarePlus, Send, LoaderCircle, CheckCircle2 } from 'lucide-react'
import { Dialog } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { submitFeedback } from '@/serverFunctions/feedback'

const EXAMPLES = [
  'Not working in Kota Bharu',
  'No bus detected at MRT Damansara',
  'Buses not showing in Ipoh',
  'Wrong ETA for RapidKL route T789',
  'App stuck loading in Kuching',
  'Bus position looks way off from actual location',
]

type FeedbackButtonProps = {
  region?: string
  className?: string
}

export function FeedbackButton({ region, className }: FeedbackButtonProps) {
  const [open, setOpen] = useState(false)
  const [message, setMessage] = useState('')
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle')
  const [errorText, setErrorText] = useState('')

  const handleOpen = () => {
    setOpen(true)
    setStatus('idle')
    setMessage('')
    setErrorText('')
  }

  const handleClose = (nextOpen: boolean) => {
    if (status === 'loading') return
    setOpen(nextOpen)
    if (!nextOpen) {
      setStatus('idle')
      setMessage('')
      setErrorText('')
    }
  }

  const handleExampleClick = (example: string) => {
    setMessage(example)
  }

  const handleSubmit = async () => {
    const trimmed = message.trim()
    if (!trimmed) {
      setErrorText('Please write something before submitting.')
      return
    }

    setStatus('loading')
    setErrorText('')

    try {
      await submitFeedback({ data: { message: trimmed, region } })
      setStatus('success')
    } catch {
      setStatus('error')
      setErrorText('Something went wrong. Please try again.')
    }
  }

  return (
    <>
      <button
        type="button"
        onClick={handleOpen}
        aria-label="Send feedback"
        className={cn(
          'group flex items-center gap-1.5 rounded-full border border-amber-200/80 bg-white/85 px-3 py-1.5 text-xs font-medium text-slate-700 shadow-md backdrop-blur-sm transition-all hover:border-amber-300 hover:bg-white hover:shadow-lg active:scale-95',
          className,
        )}
      >
        <MessageSquarePlus className="h-3.5 w-3.5 text-amber-600 transition-transform group-hover:scale-110" />
        Feedback
      </button>

      <Dialog
        open={open}
        onOpenChange={handleClose}
        title="Send Feedback"
        description="Found a bug or something off? Let us know — we read every message."
      >
        {status === 'success' ? (
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            <CheckCircle2 className="h-10 w-10 text-emerald-500" />
            <p className="text-sm font-medium text-slate-800">Thanks for your feedback!</p>
            <p className="text-xs text-slate-500">We'll look into it and make things better.</p>
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => handleClose(false)}
              className="mt-2"
            >
              Close
            </Button>
          </div>
        ) : (
          <div className="space-y-4">
            <div>
              <p className="mb-2 text-xs font-medium text-slate-600">Examples</p>
              <div className="flex flex-wrap gap-1.5">
                {EXAMPLES.map((example) => (
                  <button
                    key={example}
                    type="button"
                    onClick={() => handleExampleClick(example)}
                    className="rounded-full border border-amber-200 bg-amber-50 px-2.5 py-1 text-xs text-amber-800 transition-colors hover:border-amber-400 hover:bg-amber-100 active:scale-95"
                  >
                    {example}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <label htmlFor="feedback-message" className="mb-1.5 block text-xs font-medium text-slate-600">
                Your feedback
              </label>
              <textarea
                id="feedback-message"
                value={message}
                onChange={(e) => {
                  setMessage(e.target.value)
                  if (errorText) setErrorText('')
                }}
                placeholder="Describe what's not working or what you'd like to see improved..."
                rows={4}
                maxLength={1000}
                className="w-full resize-none rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-800 placeholder:text-slate-400 focus:border-amber-400 focus:outline-none focus:ring-2 focus:ring-amber-200 disabled:opacity-50"
                disabled={status === 'loading'}
              />
              <div className="mt-1 flex items-center justify-between">
                {errorText ? (
                  <p className="text-xs text-rose-600">{errorText}</p>
                ) : (
                  <span />
                )}
                <p className="ml-auto text-xs text-slate-400">{message.length}/1000</p>
              </div>
            </div>

            <Button
              type="button"
              onClick={handleSubmit}
              disabled={status === 'loading' || !message.trim()}
              className="w-full bg-amber-500 text-slate-900 hover:bg-amber-400 disabled:opacity-50"
            >
              {status === 'loading' ? (
                <>
                  <LoaderCircle className="animate-spin" />
                  Sending...
                </>
              ) : (
                <>
                  <Send />
                  Send feedback
                </>
              )}
            </Button>
          </div>
        )}
      </Dialog>
    </>
  )
}
