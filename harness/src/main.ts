// Qliphoth Test Harness
// TypeScript mock of the docs-platform for E2E testing
// This simulates the expected Sigil WASM output

import { Router, routes } from './router'
import { renderApp } from './components/App'

// Initialize the application
document.addEventListener('DOMContentLoaded', () => {
  const root = document.getElementById('root')
  if (!root) return

  // Initialize router
  const router = new Router(routes)

  // Render initial app
  renderApp(root, router)

  // Handle navigation
  router.onNavigate(() => {
    renderApp(root, router)
  })
})
