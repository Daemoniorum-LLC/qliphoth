// Header component

export function renderHeader(): string {
  return `
    <header class="site-header" data-testid="header">
      <div class="header-content">
        <a href="/" class="header-brand" data-testid="logo">
          <span class="brand-logo">☿</span>
          <span class="brand-name">Daemoniorum</span>
        </a>

        <nav class="header-nav" data-testid="nav">
          <a href="/docs" class="nav-link" data-testid="nav-docs">Docs</a>
          <a href="/api" class="nav-link" data-testid="nav-api">API</a>
          <a href="/guides" class="nav-link" data-testid="nav-guides">Guides</a>
          <a href="/examples" class="nav-link" data-testid="nav-examples">Examples</a>
          <a href="/playground" class="nav-link" data-testid="nav-playground">Playground</a>
          <a
            href="https://github.com/Daemoniorum-LLC"
            class="nav-link"
            target="_blank"
            rel="noopener"
            data-testid="nav-github"
          >
            GitHub
          </a>
        </nav>

        <div class="header-actions">
          <button
            class="search-trigger"
            data-testid="search-btn"
            aria-label="Search"
          >
            <span class="search-icon">🔍</span>
            <span class="search-shortcut">⌘K</span>
          </button>

          <button
            class="theme-toggle"
            data-testid="theme-toggle"
            aria-label="Toggle theme"
          >
            <span class="theme-icon">◐</span>
          </button>

          <button
            class="mobile-menu-toggle"
            data-testid="mobile-menu-btn"
            aria-label="Toggle menu"
          >
            <span class="menu-icon">☰</span>
          </button>
        </div>
      </div>
    </header>
  `
}
