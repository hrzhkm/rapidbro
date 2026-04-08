import { createServerFn } from '@tanstack/react-start'
import { deleteCookie, getSession, useSession } from '@tanstack/react-start/server'
import { prisma } from '@/db'

function sessionConfig() {
  const secret = process.env.SESSION_SECRET
  if (!secret || secret.length < 32) {
    console.error('SERVER CONFIG ERROR: SESSION_SECRET must be set and at least 32 characters long.')
    throw new Error('Authentication unavailable.')
  }
  return {
    password: secret,
    name: 'admin_session',
    maxAge: 60 * 60 * 8, // 8 hours
    cookie: {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax' as const,
      path: '/',
    },
  }
}

export const adminLogin = createServerFn({ method: 'POST' })
  .inputValidator((data: unknown) => {
    if (
      typeof data !== 'object' ||
      data === null ||
      typeof (data as Record<string, unknown>).username !== 'string' ||
      typeof (data as Record<string, unknown>).password !== 'string'
    ) {
      throw new Error('Invalid request.')
    }
    return data as { username: string; password: string }
  })
  .handler(async ({ data }) => {
    const adminUsername = process.env.ADMIN_USERNAME
    const adminPassword = process.env.ADMIN_PASSWORD

    if (!adminUsername || !adminPassword) {
      console.error('SERVER CONFIG ERROR: ADMIN_USERNAME or ADMIN_PASSWORD not configured.')
      throw new Error('Authentication unavailable.')
    }

    if (data.username !== adminUsername || data.password !== adminPassword) {
      throw new Error('Invalid username or password.')
    }

    const session = await useSession<{ authenticated: boolean }>(sessionConfig())
    await session.update({ authenticated: true })

    return { ok: true }
  },
)

export const checkAdminAuth = createServerFn({ method: 'GET' }).handler(async () => {
  try {
    const session = await getSession<{ authenticated: boolean }>(sessionConfig())
    return session.data.authenticated === true
  } catch {
    return false
  }
})

export const adminLogout = createServerFn({ method: 'POST' }).handler(async () => {
  try {
    const session = await useSession<{ authenticated: boolean }>(sessionConfig())
    await session.clear()
  } catch {
    // ignore — clear the cookie regardless
  }
  deleteCookie('admin_session')
  return { ok: true }
})

export const getFeedback = createServerFn({ method: 'GET' }).handler(async () => {
  const isAuth = await (async () => {
    try {
      const session = await getSession<{ authenticated: boolean }>(sessionConfig())
      return session.data.authenticated === true
    } catch {
      return false
    }
  })()

  if (!isAuth) throw new Error('Unauthorized')

  const feedback = await prisma.feedback.findMany({
    orderBy: { createdAt: 'desc' },
  })
  return feedback
})

export const deleteFeedback = createServerFn({ method: 'POST' })
  .inputValidator((data: unknown) => {
    if (
      typeof data !== 'object' ||
      data === null ||
      typeof (data as Record<string, unknown>).id !== 'number'
    ) {
      throw new Error('Invalid request.')
    }
    return data as { id: number }
  })
  .handler(async ({ data }) => {
    const isAuth = await (async () => {
      try {
        const session = await getSession<{ authenticated: boolean }>(sessionConfig())
        return session.data.authenticated === true
      } catch {
        return false
      }
    })()

    if (!isAuth) throw new Error('Unauthorized')

    await prisma.feedback.delete({ where: { id: data.id } })
    return { ok: true }
  },
)
