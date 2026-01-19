// App component - renders the docs-platform layout
import { Router } from '../router'
import { renderHeader } from './Header'
import { renderSidebar } from './Sidebar'
import { renderHome } from './pages/Home'
import { renderDocsIndex } from './pages/DocsIndex'
import { renderDocPage } from './pages/DocPage'
import { renderPlayground } from './pages/Playground'
import { renderNotFound } from './pages/NotFound'

let currentTheme: 'light' | 'dark' = 'dark'

export function toggleTheme() {
  currentTheme = currentTheme === 'dark' ? 'light' : 'dark'
  document.documentElement.setAttribute('data-theme', currentTheme)
}

export function getTheme() {
  return currentTheme
}

export function renderApp(root: HTMLElement, router: Router) {
  const { component, params } = router.currentRoute

  // Determine if we need a sidebar layout
  const needsSidebar = ['DocsIndex', 'DocPage', 'ApiIndex', 'ApiReference'].includes(component)
  const isLanding = component === 'Home'
  const isPlayground = component === 'Playground'

  let content = ''

  // Render the appropriate page
  switch (component) {
    case 'Home':
      content = renderHome()
      break
    case 'DocsIndex':
      content = renderDocsIndex()
      break
    case 'DocPage':
      content = renderDocPage(params || {})
      break
    case 'Playground':
      content = renderPlayground()
      break
    case 'NotFound':
    default:
      content = renderNotFound()
      break
  }

  // Build the full page layout
  if (isLanding) {
    root.innerHTML = `
      <div class="landing-layout theme-${currentTheme}" data-testid="landing-layout">
        ${renderHeader()}
        <main class="landing-content" data-testid="landing-content">
          ${content}
        </main>
        ${renderFooter()}
      </div>
    `
  } else if (isPlayground) {
    root.innerHTML = `
      <div class="playground-layout theme-${currentTheme}" data-testid="playground-layout">
        ${renderHeader()}
        <main class="playground-main" data-testid="playground-main">
          ${content}
        </main>
      </div>
    `
  } else if (needsSidebar) {
    root.innerHTML = `
      <div class="docs-layout theme-${currentTheme}" data-testid="docs-layout">
        ${renderHeader()}
        <div class="docs-main">
          ${renderSidebar(router.params.project)}
          <main class="docs-content" data-testid="docs-content">
            ${content}
          </main>
        </div>
        ${renderFooter()}
      </div>
    `
  } else {
    root.innerHTML = `
      <div class="default-layout theme-${currentTheme}" data-testid="default-layout">
        ${renderHeader()}
        <main class="main-content">
          ${content}
        </main>
        ${renderFooter()}
      </div>
    `
  }

  // Attach event listeners
  attachEventListeners()
}

function renderFooter(): string {
  return `
    <footer class="site-footer" data-testid="footer">
      <div class="footer-content">
        <div class="footer-brand">
          <span class="footer-logo">☿</span>
          <span>Daemoniorum</span>
        </div>
        <nav class="footer-links">
          <a href="/docs">Documentation</a>
          <a href="/api">API Reference</a>
          <a href="https://github.com/Daemoniorum-LLC" target="_blank">GitHub</a>
        </nav>
        <div class="footer-copyright">
          © 2025 Daemoniorum, LLC
        </div>
      </div>
    </footer>
  `
}

function attachEventListeners() {
  // Theme toggle
  const themeToggle = document.querySelector('[data-testid="theme-toggle"]')
  if (themeToggle) {
    themeToggle.addEventListener('click', () => {
      toggleTheme()
      const root = document.getElementById('root')
      const layout = root?.querySelector('[class*="-layout"]')
      if (layout) {
        layout.className = layout.className.replace(/theme-\w+/, `theme-${currentTheme}`)
      }
    })
  }

  // Mobile menu toggle
  const menuToggle = document.querySelector('[data-testid="mobile-menu-btn"]')
  const sidebar = document.querySelector('[data-testid="sidebar"]')
  if (menuToggle && sidebar) {
    menuToggle.addEventListener('click', () => {
      sidebar.classList.toggle('sidebar--open')
    })
  }

  // Search modal
  const searchTrigger = document.querySelector('[data-testid="search-btn"]')
  if (searchTrigger) {
    searchTrigger.addEventListener('click', () => {
      openSearchModal()
    })
  }

  // Sidebar group expand/collapse
  const groupHeaders = document.querySelectorAll('.nav-group-header')
  groupHeaders.forEach(header => {
    header.addEventListener('click', () => {
      const group = header.closest('.nav-group')
      const content = group?.querySelector('.nav-group-content') as HTMLElement
      if (content) {
        const isVisible = content.style.display !== 'none'
        content.style.display = isVisible ? 'none' : 'block'
        const chevron = header.querySelector('.nav-group-chevron')
        if (chevron) {
          chevron.textContent = isVisible ? '▶' : '▼'
        }
      }
    })
  })

  // Output panel tabs
  const outputTabs = document.querySelectorAll('[data-testid="output-tabs"] .panel-tab')
  outputTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      outputTabs.forEach(t => t.classList.remove('panel-tab--active'))
      tab.classList.add('panel-tab--active')
    })
  })

  // Keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    // Cmd/Ctrl + K for search
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault()
      openSearchModal()
    }
  })
}

function openSearchModal() {
  const existing = document.querySelector('[data-testid="search-modal"]')
  if (existing) return

  const modal = document.createElement('div')
  modal.className = 'search-modal'
  modal.setAttribute('data-testid', 'search-modal')
  modal.innerHTML = `
    <div class="search-modal-overlay" data-testid="search-overlay"></div>
    <div class="search-modal-content">
      <input
        type="text"
        class="search-input"
        placeholder="Search documentation..."
        data-testid="search-input"
        autofocus
      />
      <div class="search-results" data-testid="search-results">
        <p class="search-hint">Type to search...</p>
      </div>
    </div>
  `

  document.body.appendChild(modal)

  // Close on overlay click
  const backdrop = modal.querySelector('[data-testid="search-overlay"]')
  backdrop?.addEventListener('click', () => modal.remove())

  // Close on Escape
  const closeOnEscape = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      modal.remove()
      document.removeEventListener('keydown', closeOnEscape)
    }
  }
  document.addEventListener('keydown', closeOnEscape)

  // Focus input
  const input = modal.querySelector('[data-testid="search-input"]') as HTMLInputElement
  setTimeout(() => input?.focus(), 0)
}
