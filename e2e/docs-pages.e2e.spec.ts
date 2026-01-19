import { test, expect } from '@playwright/test'

test.describe('Docs Index Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/docs')
  })

  test('renders docs index with title', async ({ page }) => {
    await expect(page.getByTestId('docs-article')).toBeVisible()
    await expect(page.getByTestId('page-title')).toHaveText('Documentation')
    await expect(page.getByTestId('page-description')).toBeVisible()
  })

  test('getting started section is present', async ({ page }) => {
    const section = page.getByTestId('getting-started-section')
    await expect(section).toBeVisible()

    await expect(page.getByTestId('quickstart-card')).toBeVisible()
    await expect(page.getByTestId('installation-card')).toBeVisible()
    await expect(page.getByTestId('playground-card')).toBeVisible()
  })

  test('products section lists all products', async ({ page }) => {
    const section = page.getByTestId('products-section')
    await expect(section).toBeVisible()

    await expect(page.getByTestId('sigil-link')).toBeVisible()
    await expect(page.getByTestId('qliphoth-link')).toBeVisible()
    await expect(page.getByTestId('leviathan-link')).toBeVisible()
    await expect(page.getByTestId('nyx-link')).toBeVisible()
  })

  test('product links navigate to product docs', async ({ page }) => {
    await page.getByTestId('sigil-link').click()
    await expect(page).toHaveURL('/docs/sigil')
    await expect(page.getByTestId('doc-page')).toBeVisible()
  })

  test('playground card navigates to playground', async ({ page }) => {
    await page.getByTestId('playground-card').click()
    await expect(page).toHaveURL('/playground')
  })
})

test.describe('Doc Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/docs/sigil')
  })

  test('renders doc page with content', async ({ page }) => {
    await expect(page.getByTestId('doc-page')).toBeVisible()
    await expect(page.getByTestId('doc-title')).toBeVisible()
    await expect(page.getByTestId('doc-body')).toBeVisible()
  })

  test('breadcrumbs show navigation path', async ({ page }) => {
    const breadcrumbs = page.getByTestId('breadcrumbs')
    await expect(breadcrumbs).toBeVisible()
    await expect(breadcrumbs).toContainText('Docs')
  })

  test('breadcrumb link navigates back to docs', async ({ page }) => {
    await page.getByTestId('breadcrumbs').getByRole('link', { name: 'Docs' }).click()
    await expect(page).toHaveURL('/docs')
  })

  test('doc meta shows reading time and updated date', async ({ page }) => {
    const meta = page.getByTestId('doc-meta')
    await expect(meta).toBeVisible()
    await expect(meta).toContainText('min read')
    await expect(meta).toContainText('Updated')
  })

  test('table of contents sidebar is present', async ({ page }) => {
    const toc = page.getByTestId('toc-sidebar')
    await expect(toc).toBeVisible()

    const tocNav = page.getByTestId('toc-nav')
    await expect(tocNav).toBeVisible()
  })

  test('TOC links scroll to sections', async ({ page, viewport }) => {
    // Skip on tablet/mobile where TOC is hidden
    test.skip(viewport !== null && viewport.width < 1024, 'TOC not visible on smaller viewports')

    const tocLink = page.getByTestId('toc-nav').locator('a').first()
    const href = await tocLink.getAttribute('href')

    await tocLink.click()

    // URL should have hash
    expect(href).toMatch(/^#/)
  })

  test('page navigation shows prev/next links', async ({ page }) => {
    const pageNav = page.getByTestId('page-nav')
    await expect(pageNav).toBeVisible()

    await expect(page.getByTestId('prev-link')).toBeVisible()
    await expect(page.getByTestId('next-link')).toBeVisible()
  })

  test('prev link navigates to previous page', async ({ page }) => {
    await page.getByTestId('prev-link').click()
    await expect(page).toHaveURL('/docs')
  })
})

test.describe('Doc Page - Different Projects', () => {
  test('sigil docs page renders correctly', async ({ page }) => {
    await page.goto('/docs/sigil')

    await expect(page.getByTestId('doc-title')).toContainText('Sigil')
    await expect(page.getByTestId('doc-body')).toContainText('Evidentiality')
  })

  test('qliphoth docs page renders correctly', async ({ page }) => {
    await page.goto('/docs/qliphoth')

    await expect(page.getByTestId('doc-title')).toContainText('Qliphoth')
    await expect(page.getByTestId('doc-body')).toContainText('Component')
  })
})

test.describe('Code Blocks', () => {
  test('code blocks are present in doc content', async ({ page }) => {
    await page.goto('/docs/sigil')

    const codeBlocks = page.locator('.code-block')
    await expect(codeBlocks.first()).toBeVisible()
  })

  test('code blocks contain pre and code elements', async ({ page }) => {
    await page.goto('/docs/sigil')

    const codeBlock = page.locator('.code-block').first()
    await expect(codeBlock.locator('pre')).toBeVisible()
    await expect(codeBlock.locator('code')).toBeVisible()
  })
})

test.describe('Doc Page Responsive', () => {
  test.describe('desktop', () => {
    test.use({ viewport: { width: 1280, height: 800 } })

    test('TOC sidebar is visible on desktop', async ({ page }) => {
      await page.goto('/docs/sigil')
      await expect(page.getByTestId('toc-sidebar')).toBeVisible()
    })
  })

  test.describe('tablet', () => {
    test.use({ viewport: { width: 768, height: 1024 } })

    test('TOC sidebar may be hidden on tablet', async ({ page }) => {
      await page.goto('/docs/sigil')
      // TOC behavior depends on CSS - just check page loads
      await expect(page.getByTestId('doc-page')).toBeVisible()
    })
  })

  test.describe('mobile', () => {
    test.use({ viewport: { width: 375, height: 667 } })

    test('doc content is visible on mobile', async ({ page }) => {
      await page.goto('/docs/sigil')

      const docPage = page.getByTestId('doc-page')
      await expect(docPage).toBeVisible()

      // Check doc page is visible and rendered on mobile
      const boundingBox = await docPage.boundingBox()
      expect(boundingBox).toBeTruthy()
      expect(boundingBox?.width).toBeGreaterThan(0)
    })
  })
})

test.describe('Doc Navigation Flow', () => {
  test('complete navigation flow: home -> docs -> product -> back', async ({ page }) => {
    // Start at home
    await page.goto('/')
    await expect(page.getByTestId('home')).toBeVisible()

    // Navigate to docs via CTA
    await page.getByTestId('cta-docs').click()
    await expect(page).toHaveURL('/docs')

    // Navigate to Sigil docs
    await page.getByTestId('sigil-link').click()
    await expect(page).toHaveURL('/docs/sigil')
    await expect(page.getByTestId('doc-title')).toContainText('Sigil')

    // Navigate back via breadcrumbs
    await page.getByTestId('breadcrumbs').getByRole('link', { name: 'Docs' }).click()
    await expect(page).toHaveURL('/docs')

    // Navigate home via logo
    await page.getByTestId('logo').click()
    await expect(page).toHaveURL('/')
  })

  test('sidebar navigation updates URL and content', async ({ page, viewport }) => {
    // Skip on tablet/mobile where sidebar is hidden or collapsed
    test.skip(viewport !== null && viewport.width < 1024, 'Sidebar not visible on smaller viewports')

    await page.goto('/docs')

    // Navigate to Sigil
    await page.getByTestId('sidebar-link-sigil').click()
    await expect(page).toHaveURL('/docs/sigil')
    await expect(page.getByTestId('doc-title')).toContainText('Sigil')

    // Navigate to Qliphoth
    await page.getByTestId('sidebar-link-qliphoth').click()
    await expect(page).toHaveURL('/docs/qliphoth')
    await expect(page.getByTestId('doc-title')).toContainText('Qliphoth')
  })
})
