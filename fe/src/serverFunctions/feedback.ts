import { createServerFn } from '@tanstack/react-start'
import { prisma } from '@/db'

type SubmitFeedbackInput = {
  message: string
  region?: string
}

export const submitFeedback = createServerFn({ method: 'POST' })
  .validator((data: SubmitFeedbackInput) => data)
  .handler(async ({ data }) => {
    const trimmed = data.message.trim()
    if (!trimmed) {
      throw new Error('Feedback message cannot be empty.')
    }
    if (trimmed.length > 1000) {
      throw new Error('Feedback message is too long (max 1000 characters).')
    }

    await prisma.feedback.create({
      data: {
        message: trimmed,
        region: data.region ?? null,
      },
    })

    return { ok: true }
  })
