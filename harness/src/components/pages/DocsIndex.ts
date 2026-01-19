// Docs index page

export function renderDocsIndex(): string {
  return `
    <article class="docs-article" data-testid="docs-article">
      <header class="doc-header">
        <h1 data-testid="page-title">Documentation</h1>
        <p class="lead" data-testid="page-description">
          Welcome to the Daemoniorum documentation. Learn how to build powerful
          applications with our ecosystem of tools and frameworks.
        </p>
      </header>

      <section class="doc-section" data-testid="getting-started-section">
        <h2>Getting Started</h2>
        <div class="card-grid">
          <a href="/docs/getting-started" class="doc-card" data-testid="quickstart-card">
            <h3>🚀 Quick Start</h3>
            <p>Get up and running in 5 minutes</p>
          </a>
          <a href="/docs/installation" class="doc-card" data-testid="installation-card">
            <h3>📦 Installation</h3>
            <p>Install Sigil and set up your environment</p>
          </a>
          <a href="/playground" class="doc-card" data-testid="playground-card">
            <h3>🎮 Playground</h3>
            <p>Try Sigil in your browser</p>
          </a>
        </div>
      </section>

      <section class="doc-section" data-testid="products-section">
        <h2>Products</h2>
        <div class="product-list">
          <a href="/docs/sigil" class="product-item" data-testid="sigil-link">
            <div class="product-icon">☉</div>
            <div class="product-info">
              <h3>Sigil Language</h3>
              <p>Polysynthetic systems programming with evidentiality types</p>
              <span class="badge badge--beta">Beta</span>
            </div>
          </a>
          <a href="/docs/qliphoth" class="product-item" data-testid="qliphoth-link">
            <div class="product-icon">🌐</div>
            <div class="product-info">
              <h3>Qliphoth</h3>
              <p>React-inspired web framework for Sigil</p>
              <span class="badge badge--alpha">Alpha</span>
            </div>
          </a>
          <a href="/docs/leviathan" class="product-item" data-testid="leviathan-link">
            <div class="product-icon">🖥️</div>
            <div class="product-info">
              <h3>Leviathan</h3>
              <p>Enterprise backend framework with GraphQL</p>
              <span class="badge badge--stable">Stable</span>
            </div>
          </a>
          <a href="/docs/nyx" class="product-item" data-testid="nyx-link">
            <div class="product-icon">🧠</div>
            <div class="product-info">
              <h3>Nyx</h3>
              <p>Autonomous AI agent framework</p>
              <span class="badge badge--beta">Beta</span>
            </div>
          </a>
        </div>
      </section>
    </article>
  `
}
