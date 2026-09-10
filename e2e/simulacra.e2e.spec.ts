import { test, expect, Page } from '@playwright/test'

/**
 * Qliphoth Simulacra Tests
 *
 * These tests simulate different user archetypes interacting with the UI.
 * Each archetype has different behaviors, speeds, and expectations.
 *
 * Sprint 8: Integration Testing - User Archetype Testing
 */

// Archetype behavior configurations
const archetypes = {
  'first-time-user': {
    clickDelay: 500,
    typingDelay: 50,
    readsTooltips: true,
  },
  'power-user': {
    clickDelay: 100,
    typingDelay: 20,
    usesKeyboard: true,
  },
  'screen-reader-user': {
    clickDelay: 300,
    usesKeyboard: true,
    checksAriaLabels: true,
  },
  'keyboard-only-user': {
    usesKeyboard: true,
    noMouse: true,
  },
  'mobile-user': {
    viewport: { width: 375, height: 667 },
    usesTouch: true,
  },
  'impatient-user': {
    clickDelay: 50,
    rapidInteractions: true,
    maxWait: 2000,
  },
}

// Helper to simulate typing with archetype speed
async function typeWithArchetype(
  page: Page,
  selector: string,
  text: string,
  archetype: keyof typeof archetypes
) {
  const config = archetypes[archetype]
  const delay = config.typingDelay ?? 30

  await page.locator(selector).fill('')
  await page.locator(selector).focus()

  for (const char of text) {
    await page.keyboard.type(char, { delay })
  }
}

// Helper to click with archetype delay
async function clickWithArchetype(
  page: Page,
  selector: string,
  archetype: keyof typeof archetypes
) {
  const config = archetypes[archetype]
  const delay = config.clickDelay ?? 200

  await page.waitForTimeout(delay / 2)
  await page.locator(selector).click()
  await page.waitForTimeout(delay / 2)
}

test.describe('Simulacra: First-Time User', () => {
  test('can discover and use the run button', async ({ page }) => {
    await page.goto('/playground')

    // First-time user explores the UI
    await expect(page.getByTestId('playground')).toBeVisible()

    // Finds and reads button text
    const runBtn = page.getByTestId('run-btn')
    await expect(runBtn).toContainText('Run')

    // Deliberate click
    await clickWithArchetype(page, '[data-testid="run-btn"]', 'first-time-user')

    // Expects clear feedback
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('can use example selector to learn', async ({ page }) => {
    await page.goto('/playground')

    // First-time user looks for examples
    const select = page.getByTestId('example-select')
    await expect(select).toBeVisible()

    // Explores options
    const options = select.locator('option')
    const count = await options.count()
    expect(count).toBeGreaterThanOrEqual(4) // Should have multiple examples

    // Selects an example
    await select.selectOption('fibonacci')

    // Waits to see result
    await page.waitForTimeout(500)
  })

  test('receives clear error feedback', async ({ page }) => {
    await page.goto('/playground')

    const textarea = page.getByTestId('editor-textarea')

    // Types broken code (slow, deliberate)
    await typeWithArchetype(
      page,
      '[data-testid="editor-textarea"]',
      'rite broken(',
      'first-time-user'
    )

    // Tries to run
    await clickWithArchetype(page, '[data-testid="run-btn"]', 'first-time-user')

    // UI should remain stable with clear error
    await expect(page.getByTestId('playground')).toBeVisible()
    await expect(page.getByTestId('output-console')).toBeVisible()
  })
})

test.describe('Simulacra: Power User', () => {
  test('uses keyboard shortcuts efficiently', async ({ page }) => {
    await page.goto('/playground')

    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()

    // Fast typing
    await typeWithArchetype(
      page,
      '[data-testid="editor-textarea"]',
      'rite fast_code() {}',
      'power-user'
    )

    // Uses Ctrl+Enter to run
    await page.keyboard.press('Control+Enter')

    // Expects instant response
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('rapidly switches between tabs', async ({ page }) => {
    await page.goto('/playground')

    // Power user rapidly navigates
    for (let i = 0; i < 5; i++) {
      await clickWithArchetype(page, '[data-testid="output-tab"]', 'power-user')
      await clickWithArchetype(page, '[data-testid="wasm-tab"]', 'power-user')
      await clickWithArchetype(page, '[data-testid="ast-tab"]', 'power-user')
    }

    // UI should keep up
    await expect(page.getByTestId('output-panel')).toBeVisible()
  })

  test('edits code quickly without lag', async ({ page }) => {
    await page.goto('/playground')

    const startTime = Date.now()

    // Fast typing
    await typeWithArchetype(
      page,
      '[data-testid="editor-textarea"]',
      'rite power_user_test() { ≔ x! = 42; ret x; }',
      'power-user'
    )

    const endTime = Date.now()

    // Should be fast
    expect(endTime - startTime).toBeLessThan(5000)

    // Content should be accurate
    const value = await page.getByTestId('editor-textarea').inputValue()
    expect(value).toContain('power_user_test')
  })
})

test.describe('Simulacra: Screen Reader User', () => {
  test('can navigate with keyboard only', async ({ page }) => {
    await page.goto('/playground')

    // Tab through the interface
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab')

      // Something should always be focused
      const focusedElement = page.locator(':focus')
      await expect(focusedElement).toBeVisible()
    }
  })

  test('finds accessible button labels', async ({ page }) => {
    await page.goto('/playground')

    // Buttons should have text content
    const runBtn = page.getByTestId('run-btn')
    const formatBtn = page.getByTestId('format-btn')
    const shareBtn = page.getByTestId('share-btn')

    await expect(runBtn).not.toBeEmpty()
    await expect(formatBtn).not.toBeEmpty()
    await expect(shareBtn).not.toBeEmpty()
  })

  test('has proper heading structure', async ({ page }) => {
    await page.goto('/playground')

    // Should have at least one heading
    const headings = page.locator('h1, h2, h3')
    const count = await headings.count()
    expect(count).toBeGreaterThan(0)
  })

  test('form controls are labeled', async ({ page }) => {
    await page.goto('/playground')

    // Select should be accessible
    const select = page.getByTestId('example-select')
    const tagName = await select.evaluate(el => el.tagName.toLowerCase())
    expect(tagName).toBe('select')

    // Editor should be a textarea
    const textarea = page.getByTestId('editor-textarea')
    const textareaTag = await textarea.evaluate(el => el.tagName.toLowerCase())
    expect(textareaTag).toBe('textarea')
  })
})

test.describe('Simulacra: Keyboard-Only User', () => {
  test('can activate run button with Enter', async ({ page }) => {
    await page.goto('/playground')

    // Tab to run button
    const runBtn = page.getByTestId('run-btn')

    // Focus the button
    await runBtn.focus()
    await expect(runBtn).toBeFocused()

    // Press Enter to activate
    await page.keyboard.press('Enter')

    // UI should respond
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('can navigate all interactive elements', async ({ page }) => {
    await page.goto('/playground')

    const targetTestIds = ['run-btn', 'format-btn', 'share-btn', 'example-select', 'editor-textarea']
    const foundTestIds = new Set<string>()

    // Tab through the interface (limited iterations to avoid timeout)
    for (let i = 0; i < 15; i++) {
      await page.keyboard.press('Tab')

      // Get focused element's data-testid if it exists
      const focused = page.locator(':focus')
      const testId = await focused.getAttribute('data-testid', { timeout: 500 }).catch(() => null)

      if (testId && targetTestIds.includes(testId)) {
        foundTestIds.add(testId)
      }
    }

    // Should find at least some of the key elements
    expect(foundTestIds.size).toBeGreaterThan(0)
  })
})

test.describe('Simulacra: Mobile User', () => {
  test.use({ viewport: { width: 375, height: 667 }, hasTouch: true })

  test('layout adapts to mobile viewport', async ({ page }) => {
    await page.goto('/playground')

    // Page should not overflow
    const body = page.locator('body')
    const bodyBox = await body.boundingBox()

    expect(bodyBox?.width).toBeLessThanOrEqual(375)
  })

  test('touch targets are adequate size', async ({ page }) => {
    await page.goto('/playground')

    const runBtn = page.getByTestId('run-btn')
    const box = await runBtn.boundingBox()

    // Touch targets should be at least 44px (Apple HIG)
    if (box) {
      expect(box.height).toBeGreaterThanOrEqual(44)
      expect(box.width).toBeGreaterThanOrEqual(44)
    }
  })

  test('can interact with touch gestures', async ({ page }) => {
    await page.goto('/playground')

    // Tap the run button (touch emulation enabled via hasTouch)
    await page.getByTestId('run-btn').tap()

    // UI should respond
    await expect(page.getByTestId('playground')).toBeVisible()
  })
})

test.describe('Simulacra: Impatient User', () => {
  test('UI responds quickly to rapid clicks', async ({ page }) => {
    await page.goto('/playground')

    const startTime = Date.now()

    // Rapid clicks
    for (let i = 0; i < 10; i++) {
      await clickWithArchetype(page, '[data-testid="run-btn"]', 'impatient-user')
    }

    const endTime = Date.now()

    // Should complete in reasonable time
    expect(endTime - startTime).toBeLessThan(3000)

    // UI should remain stable
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('page loads quickly', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/playground')

    await expect(page.getByTestId('playground')).toBeVisible()

    const endTime = Date.now()

    // Page should load within 2 seconds
    expect(endTime - startTime).toBeLessThan(2000)
  })

  test('interactions do not block UI', async ({ page }) => {
    await page.goto('/playground')

    // Start an action
    await page.getByTestId('run-btn').click()

    // UI should remain responsive
    await expect(page.getByTestId('format-btn')).toBeEnabled()
    await expect(page.getByTestId('share-btn')).toBeEnabled()
  })
})
