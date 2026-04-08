import { createFileRoute, redirect } from '@tanstack/react-router'
import { checkAdminAuth } from '@/serverFunctions/admin'

export const Route = createFileRoute('/admin/')({
  beforeLoad: async () => {
    const isAuth = await checkAdminAuth()
    throw redirect({ to: isAuth ? '/admin/dashboard' : '/admin/login' })
  },
  component: () => null,
})
