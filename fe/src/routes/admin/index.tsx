import { createFileRoute, Navigate } from '@tanstack/react-router'

export const Route = createFileRoute('/admin/')({
  component: RouteComponent,
})

function RouteComponent() {
  const isAuth = typeof window !== 'undefined' && sessionStorage.getItem('admin_authenticated') === 'true'
  return <Navigate to={isAuth ? '/admin/dashboard' : '/admin/login'} />
}
