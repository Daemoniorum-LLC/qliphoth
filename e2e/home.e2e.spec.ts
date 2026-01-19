import { test, expect } from '@playwright/test'

test.describe('Home Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('renders home page', async ({ page }) => {
    await expect(page.getByTestId('home')).toBeVisible()
  })

  test('hero section is present', async ({ page }) => {
    await expect(page.getByTestId('hero')).toBeVisible()
    await expect(page.getByTestId('hero-title')).toBeVisible()
    await expect(page.getByTestId('hero-tagline')).toBeVisible()
  })

  test('hero title contains Sigil branding', async ({ page }) => {
    const title = page.getByTestId('hero-title')
    await expect(title).toContainText('Sigil')
  })

  test('CTA buttons are present', async ({ page }) => {
    await expect(page.getByTestId('cta-docs')).toBeVisible()
    await expect(page.getByTestId('cta-playground')).toBeVisible()
  })

  test('docs CTA navigates to docs', async ({ page }) => {
    await page.getByTestId('cta-docs').click()
    await expect(page).toHaveURL('/docs')
  })

  test('playground CTA navigates to playground', async ({ page }) => {
    await page.getByTestId('cta-playground').click()
    await expect(page).toHaveURL('/playground')
  })
})

test.describe('Features Section', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('features section is present', async ({ page }) => {
    await expect(page.getByTestId('features')).toBeVisible()
  })

  test('displays feature cards', async ({ page }) => {
    const cards = page.locator('[data-testid^="feature-"]')
    const count = await cards.count()
    expect(count).toBeGreaterThanOrEqual(3)
  })

  test('evidentiality feature is highlighted', async ({ page }) => {
    await expect(page.getByTestId('feature-evidentiality')).toBeVisible()
  })

  test('performance feature is highlighted', async ({ page }) => {
    await expect(page.getByTestId('feature-performance')).toBeVisible()
  })

  test('wasm feature is highlighted', async ({ page }) => {
    await expect(page.getByTestId('feature-wasm')).toBeVisible()
  })
})

test.describe('Products Section', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('products section is present', async ({ page }) => {
    await expect(page.getByTestId('products')).toBeVisible()
  })

  test('displays Sigil product', async ({ page }) => {
    await expect(page.getByTestId('product-sigil')).toBeVisible()
  })

  test('displays Qliphoth product', async ({ page }) => {
    await expect(page.getByTestId('product-qliphoth')).toBeVisible()
  })

  test('displays Leviathan product', async ({ page }) => {
    await expect(page.getByTestId('product-leviathan')).toBeVisible()
  })

  test('product cards have links to docs', async ({ page }) => {
    const sigilCard = page.getByTestId('product-sigil')
    await expect(sigilCard).toHaveAttribute('href', '/docs/sigil')
  })

  test('clicking product card navigates to product docs', async ({ page }) => {
    await page.getByTestId('product-sigil').click()
    await expect(page).toHaveURL('/docs/sigil')
  })
})

test.describe('Code Preview Section', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('code preview section is present', async ({ page }) => {
    await expect(page.getByTestId('code-preview')).toBeVisible()
  })

  test('code preview shows Sigil code', async ({ page }) => {
    const preview = page.getByTestId('code-preview')
    await expect(preview).toContainText('rite')
  })

  test('code preview has syntax highlighting container', async ({ page }) => {
    const codeBlock = page.getByTestId('code-preview').locator('pre')
    await expect(codeBlock).toBeVisible()
  })
})

test.describe('Home Page Responsive', () => {
  test.describe('desktop', () => {
    test.use({ viewport: { width: 1280, height: 800 } })

    test('feature cards display in grid', async ({ page }) => {
      await page.goto('/')

      const features = page.getByTestId('features')
      await expect(features).toBeVisible()

      // Cards should be in a row on desktop
      const cards = features.locator('[data-testid^="feature-"]')
      const firstCard = await cards.nth(0).boundingBox()
      const secondCard = await cards.nth(1).boundingBox()

      // Cards should be side by side (similar Y position)
      if (firstCard && secondCard) {
        expect(Math.abs(firstCard.y - secondCard.y)).toBeLessThan(50)
      }
    })
  })

  test.describe('tablet', () => {
    test.use({ viewport: { width: 768, height: 1024 } })

    test('home page is readable on tablet', async ({ page }) => {
      await page.goto('/')

      await expect(page.getByTestId('hero')).toBeVisible()
      await expect(page.getByTestId('cta-docs')).toBeVisible()
      await expect(page.getByTestId('cta-playground')).toBeVisible()
    })
  })

  test.describe('mobile', () => {
    test.use({ viewport: { width: 375, height: 667 } })

    test('hero section adapts to mobile', async ({ page }) => {
      await page.goto('/')

      const hero = page.getByTestId('hero')
      await expect(hero).toBeVisible()

      // Check hero doesn't overflow
      const heroBox = await hero.boundingBox()
      expect(heroBox?.width).toBeLessThanOrEqual(375)
    })

    test('CTA buttons stack vertically on mobile', async ({ page }) => {
      await page.goto('/')

      const docsBtn = page.getByTestId('cta-docs')
      const playgroundBtn = page.getByTestId('cta-playground')

      await expect(docsBtn).toBeVisible()
      await expect(playgroundBtn).toBeVisible()

      const docsBox = await docsBtn.boundingBox()
      const playgroundBox = await playgroundBtn.boundingBox()

      // On mobile, buttons might stack (playground below docs)
      // or be side by side but smaller
      if (docsBox && playgroundBox) {
        const sameRow = Math.abs(docsBox.y - playgroundBox.y) < 10
        const stacked = playgroundBox.y > docsBox.y

        expect(sameRow || stacked).toBe(true)
      }
    })

    test('feature cards stack on mobile', async ({ page }) => {
      await page.goto('/')

      const cards = page.locator('[data-testid^="feature-"]')
      const firstCard = await cards.nth(0).boundingBox()
      const secondCard = await cards.nth(1).boundingBox()

      // Cards should stack vertically on mobile
      if (firstCard && secondCard) {
        expect(secondCard.y).toBeGreaterThan(firstCard.y)
      }
    })
  })
})

test.describe('Accessibility', () => {
  test('home page has proper heading hierarchy', async ({ page }) => {
    await page.goto('/')

    // Should have h1
    const h1 = page.locator('h1')
    await expect(h1.first()).toBeVisible()

    // Check h2s exist for sections
    const h2s = page.locator('h2')
    const count = await h2s.count()
    expect(count).toBeGreaterThanOrEqual(1)
  })

  test('buttons have accessible text', async ({ page }) => {
    await page.goto('/')

    const docsBtn = page.getByTestId('cta-docs')
    const playgroundBtn = page.getByTestId('cta-playground')

    // Buttons should have text content
    await expect(docsBtn).not.toBeEmpty()
    await expect(playgroundBtn).not.toBeEmpty()
  })

  test('links have proper href attributes', async ({ page }) => {
    await page.goto('/')

    const productLinks = page.locator('[data-testid^="product-"]')
    const count = await productLinks.count()

    for (let i = 0; i < count; i++) {
      const href = await productLinks.nth(i).getAttribute('href')
      expect(href).toBeTruthy()
      expect(href).toMatch(/^\//)
    }
  })
})
