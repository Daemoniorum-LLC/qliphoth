import { test, expect } from '@playwright/test'

test.describe('Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('renders header with logo and navigation', async ({ page, viewport }) => {
    const header = page.getByTestId('header')
    await expect(header).toBeVisible()

    const logo = page.getByTestId('logo')
    await expect(logo).toBeVisible()
    await expect(logo).toContainText('Daemoniorum')

    // Nav is hidden on mobile (uses hamburger menu instead)
    if (viewport === null || viewport.width >= 768) {
      const nav = page.getByTestId('nav')
      await expect(nav).toBeVisible()
    }
  })

  test('navigation links are present', async ({ page, viewport }) => {
    // Skip on mobile where nav is hidden
    test.skip(viewport !== null && viewport.width < 768, 'Nav hidden on mobile')

    await expect(page.getByTestId('nav-docs')).toBeVisible()
    await expect(page.getByTestId('nav-playground')).toBeVisible()
    await expect(page.getByTestId('nav-github')).toBeVisible()
  })

  test('clicking docs link navigates to docs page', async ({ page, viewport }) => {
    // Skip on mobile where nav is hidden
    test.skip(viewport !== null && viewport.width < 768, 'Nav hidden on mobile')

    await page.getByTestId('nav-docs').click()
    await expect(page).toHaveURL('/docs')
    await expect(page.getByTestId('docs-article')).toBeVisible()
  })

  test('clicking playground link navigates to playground', async ({ page, viewport }) => {
    // Skip on mobile where nav is hidden
    test.skip(viewport !== null && viewport.width < 768, 'Nav hidden on mobile')

    await page.getByTestId('nav-playground').click()
    await expect(page).toHaveURL('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('clicking logo navigates to home', async ({ page }) => {
    await page.goto('/docs')
    await page.getByTestId('logo').click()
    await expect(page).toHaveURL('/')
    await expect(page.getByTestId('home')).toBeVisible()
  })
})

test.describe('Sidebar', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/docs')
  })

  test('renders sidebar with navigation groups', async ({ page, viewport }) => {
    // Skip on mobile where sidebar is off-canvas
    test.skip(viewport !== null && viewport.width < 768, 'Sidebar off-canvas on mobile')

    const sidebar = page.getByTestId('sidebar')
    await expect(sidebar).toBeVisible()

    await expect(page.getByTestId('nav-group-getting-started')).toBeVisible()
    await expect(page.getByTestId('nav-group-products')).toBeVisible()
    await expect(page.getByTestId('nav-group-reference')).toBeVisible()
  })

  test('sidebar links navigate to correct pages', async ({ page, viewport }) => {
    // Skip on mobile where sidebar is off-canvas
    test.skip(viewport !== null && viewport.width < 768, 'Sidebar off-canvas on mobile')

    await page.getByTestId('sidebar-link-sigil').click()
    await expect(page).toHaveURL('/docs/sigil')
    await expect(page.getByTestId('doc-page')).toBeVisible()
  })

  test('sidebar link for current page is present', async ({ page, viewport }) => {
    // Skip on mobile where sidebar is off-canvas
    test.skip(viewport !== null && viewport.width < 768, 'Sidebar off-canvas on mobile')

    await page.goto('/docs/sigil')
    const sigilLink = page.getByTestId('sidebar-link-sigil')
    await expect(sigilLink).toBeVisible()
    await expect(sigilLink).toHaveAttribute('href', '/docs/sigil')
  })

  test('sidebar groups can be expanded/collapsed', async ({ page, viewport }) => {
    // Skip on mobile where sidebar is off-canvas
    test.skip(viewport !== null && viewport.width < 768, 'Sidebar off-canvas on mobile')

    const productsGroup = page.getByTestId('nav-group-products')
    const groupHeader = productsGroup.locator('.nav-group-header')
    const groupContent = productsGroup.locator('.nav-group-content')

    await expect(groupContent).toBeVisible()
    await groupHeader.click()
    // After click, content should be hidden (display: none)
    await expect(groupContent).toHaveCSS('display', 'none')
    await groupHeader.click()
    // After second click, content should be visible again
    await expect(groupContent).toHaveCSS('display', 'block')
  })
})

test.describe('Theme Toggle', () => {
  test('theme toggle button is present', async ({ page }) => {
    await page.goto('/')
    const themeToggle = page.getByTestId('theme-toggle')
    await expect(themeToggle).toBeVisible()
  })

  test('clicking theme toggle changes theme', async ({ page }) => {
    await page.goto('/')
    const html = page.locator('html')

    // Check initial theme
    const initialTheme = await html.getAttribute('data-theme')

    // Toggle theme
    await page.getByTestId('theme-toggle').click()

    // Theme should change
    const newTheme = await html.getAttribute('data-theme')
    expect(newTheme).not.toBe(initialTheme)
  })

  test('theme preference persists across navigation', async ({ page, viewport }) => {
    // Skip on mobile where nav is hidden
    test.skip(viewport !== null && viewport.width < 768, 'Nav hidden on mobile')

    await page.goto('/')

    // Toggle theme
    await page.getByTestId('theme-toggle').click()

    // Get current theme
    const html = page.locator('html')
    const themeAfterToggle = await html.getAttribute('data-theme')

    // Navigate to another page
    await page.getByTestId('nav-docs').click()

    // Theme should persist (same as after toggle)
    await expect(html).toHaveAttribute('data-theme', themeAfterToggle ?? 'dark')
  })
})

test.describe('Search Modal', () => {
  test('search button opens search modal', async ({ page }) => {
    await page.goto('/')

    await page.getByTestId('search-btn').click()

    const modal = page.getByTestId('search-modal')
    await expect(modal).toBeVisible()
  })

  test('search modal can be closed with escape', async ({ page }) => {
    await page.goto('/')

    await page.getByTestId('search-btn').click()
    await expect(page.getByTestId('search-modal')).toBeVisible()

    await page.keyboard.press('Escape')
    await expect(page.getByTestId('search-modal')).toBeHidden()
  })

  test('search modal overlay is present', async ({ page }) => {
    await page.goto('/')

    await page.getByTestId('search-btn').click()
    await expect(page.getByTestId('search-modal')).toBeVisible()

    // Verify overlay element exists in DOM
    const overlay = page.getByTestId('search-overlay')
    await expect(overlay).toBeAttached()

    // Close with Escape
    await page.keyboard.press('Escape')
    await expect(page.getByTestId('search-modal')).toBeHidden()
  })

  test('keyboard shortcut opens search modal', async ({ page }) => {
    await page.goto('/')

    await page.keyboard.press('Meta+k')
    await expect(page.getByTestId('search-modal')).toBeVisible()
  })

  test('search input receives focus when modal opens', async ({ page }) => {
    await page.goto('/')

    await page.getByTestId('search-btn').click()

    const searchInput = page.getByTestId('search-input')
    await expect(searchInput).toBeFocused()
  })
})

test.describe('Mobile Navigation', () => {
  test.use({ viewport: { width: 375, height: 667 } })

  test('mobile menu button is visible on small screens', async ({ page }) => {
    await page.goto('/')

    const menuBtn = page.getByTestId('mobile-menu-btn')
    await expect(menuBtn).toBeVisible()
  })

  test('mobile menu opens sidebar', async ({ page }) => {
    await page.goto('/docs')

    const sidebar = page.getByTestId('sidebar')
    // On mobile, sidebar starts visible but collapsed via CSS
    // Clicking menu button toggles the sidebar--open class
    await page.getByTestId('mobile-menu-btn').click()
    await expect(sidebar).toHaveClass(/sidebar--open/)
  })

  test('main navigation is hidden on mobile', async ({ page }) => {
    await page.goto('/')

    const nav = page.getByTestId('nav')
    await expect(nav).toBeHidden()
  })
})

test.describe('404 Page', () => {
  test('unknown route shows 404 page', async ({ page }) => {
    await page.goto('/unknown-page')

    await expect(page.getByTestId('not-found')).toBeVisible()
    await expect(page.getByTestId('error-code')).toHaveText('404')
    await expect(page.getByTestId('error-title')).toHaveText('Page Not Found')
  })

  test('404 page has home link', async ({ page }) => {
    await page.goto('/unknown-page')

    const homeLink = page.getByTestId('home-link')
    await expect(homeLink).toBeVisible()

    await homeLink.click()
    await expect(page).toHaveURL('/')
  })

  test('404 page has docs link', async ({ page }) => {
    await page.goto('/unknown-page')

    const docsLink = page.getByTestId('docs-link')
    await expect(docsLink).toBeVisible()

    await docsLink.click()
    await expect(page).toHaveURL('/docs')
  })
})
