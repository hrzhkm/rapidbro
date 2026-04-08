import { createServerFn } from '@tanstack/react-start'
import { prisma } from '@/db'

export const adminLogin = createServerFn({ method: 'POST' }).handler(
  // @ts-expect-error -- data is passed at runtime via { data } but typed as undefined without .validator()
  async ({ data }: { data: { username: string; password: string } }) => {
    const adminUsername = process.env.ADMIN_USERNAME
    const adminPassword = process.env.ADMIN_PASSWORD

    if (!adminUsername || !adminPassword) {
      throw new Error('Admin credentials not configured on server.')
    }

    if (data.username !== adminUsername || data.password !== adminPassword) {
      throw new Error('Invalid username or password.')
    }

    return { ok: true }
  },
)

export const getFeedback = createServerFn({ method: 'GET' }).handler(async () => {
  const feedback = await prisma.feedback.findMany({
    orderBy: { createdAt: 'desc' },
  })
  return feedback
})

export const deleteFeedback = createServerFn({ method: 'POST' }).handler(
  // @ts-expect-error -- data is passed at runtime via { data } but typed as undefined without .validator()
  async ({ data }: { data: { id: number } }) => {
    await prisma.feedback.delete({ where: { id: data.id } })
    return { ok: true }
  },
)
