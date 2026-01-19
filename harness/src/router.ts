// Simple router for the test harness

export interface Route {
  path: string
  component: string
  params?: Record<string, string>
}

export const routes: Route[] = [
  { path: '/', component: 'Home' },
  { path: '/docs', component: 'DocsIndex' },
  { path: '/docs/:project', component: 'DocPage' },
  { path: '/docs/:project/:section', component: 'DocPage' },
  { path: '/docs/:project/:section/:doc', component: 'DocPage' },
  { path: '/api', component: 'ApiIndex' },
  { path: '/api/:product', component: 'ApiReference' },
  { path: '/guides', component: 'GuidesIndex' },
  { path: '/examples', component: 'ExamplesIndex' },
  { path: '/playground', component: 'Playground' },
  { path: '/search', component: 'SearchResults' },
  { path: '/changelog', component: 'Changelog' },
  { path: '*', component: 'NotFound' },
]

export class Router {
  private routes: Route[]
  private listeners: (() => void)[] = []
  public currentRoute: Route
  public params: Record<string, string> = {}

  constructor(routes: Route[]) {
    this.routes = routes
    this.currentRoute = this.matchRoute(window.location.pathname)

    // Handle browser navigation
    window.addEventListener('popstate', () => {
      this.currentRoute = this.matchRoute(window.location.pathname)
      this.notify()
    })

    // Handle link clicks
    document.addEventListener('click', (e) => {
      const target = e.target as HTMLElement
      const link = target.closest('a')
      if (link && link.href.startsWith(window.location.origin)) {
        e.preventDefault()
        this.navigate(link.pathname)
      }
    })
  }

  private matchRoute(pathname: string): Route {
    for (const route of this.routes) {
      const match = this.matchPath(route.path, pathname)
      if (match) {
        this.params = match.params
        return { ...route, params: match.params }
      }
    }
    return { path: '*', component: 'NotFound' }
  }

  private matchPath(pattern: string, pathname: string): { params: Record<string, string> } | null {
    if (pattern === '*') return { params: {} }

    const patternParts = pattern.split('/').filter(Boolean)
    const pathParts = pathname.split('/').filter(Boolean)

    if (patternParts.length !== pathParts.length) return null

    const params: Record<string, string> = {}

    for (let i = 0; i < patternParts.length; i++) {
      if (patternParts[i].startsWith(':')) {
        params[patternParts[i].slice(1)] = pathParts[i]
      } else if (patternParts[i] !== pathParts[i]) {
        return null
      }
    }

    return { params }
  }

  navigate(pathname: string) {
    window.history.pushState({}, '', pathname)
    this.currentRoute = this.matchRoute(pathname)
    this.notify()
  }

  onNavigate(listener: () => void) {
    this.listeners.push(listener)
  }

  private notify() {
    this.listeners.forEach(l => l())
  }
}
