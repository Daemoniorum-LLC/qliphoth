// 404 Not Found page

export function renderNotFound(): string {
  return `
    <div class="not-found" data-testid="not-found">
      <div class="not-found-content">
        <h1 class="not-found-code" data-testid="error-code">404</h1>
        <h2 class="not-found-title" data-testid="error-title">Page Not Found</h2>
        <p class="not-found-message" data-testid="error-message">
          The page you're looking for doesn't exist or has been moved.
        </p>
        <div class="not-found-actions">
          <a href="/" class="btn btn-primary" data-testid="home-link">
            Go Home
          </a>
          <a href="/docs" class="btn btn-secondary" data-testid="docs-link">
            Browse Docs
          </a>
        </div>
      </div>
    </div>
  `
}
