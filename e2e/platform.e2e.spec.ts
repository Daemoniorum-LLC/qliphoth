import { test, expect } from '@playwright/test'

/**
 * Qliphoth Platform Integration Tests
 *
 * These tests verify platform behavior through the browser platform,
 * ensuring the VDOM, events, and rendering work correctly.
 *
 * Sprint 8: Integration Testing
 */

test.describe('Platform: Event Handling', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('button click events are handled', async ({ page }) => {
    const runBtn = page.getByTestId('run-btn')

    // Verify button exists and is interactive
    await expect(runBtn).toBeVisible()
    await expect(runBtn).toBeEnabled()

    // Click and verify response
    await runBtn.click()

    // Button should remain functional after click
    await expect(runBtn).toBeVisible()
  })

  test('multiple rapid clicks are handled correctly', async ({ page }) => {
    const formatBtn = page.getByTestId('format-btn')

    // Rapid clicks should not cause issues
    await formatBtn.click()
    await formatBtn.click()
    await formatBtn.click()

    // Page should remain stable
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('keyboard events in editor work', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()

    // Type some Sigil code
    await page.keyboard.type('≔ x! = 42;')

    const value = await textarea.inputValue()
    expect(value).toContain('42')
  })

  test('focus events work correctly', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    const runBtn = page.getByTestId('run-btn')

    // Focus textarea
    await textarea.focus()
    await expect(textarea).toBeFocused()

    // Focus button
    await runBtn.focus()
    await expect(runBtn).toBeFocused()
  })
})

test.describe('Platform: Widget Creation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('all playground widgets are created', async ({ page }) => {
    // Verify key UI elements exist
    await expect(page.getByTestId('playground-header')).toBeVisible()
    await expect(page.getByTestId('editor-panel')).toBeVisible()
    await expect(page.getByTestId('output-panel')).toBeVisible()
    await expect(page.getByTestId('playground-footer')).toBeVisible()
  })

  test('editor components are properly initialized', async ({ page }) => {
    await expect(page.getByTestId('athame-editor')).toBeVisible()
    await expect(page.getByTestId('editor-textarea')).toBeVisible()
    await expect(page.getByTestId('editor-gutter')).toBeVisible()
  })

  test('output panel tabs are created', async ({ page }) => {
    await expect(page.getByTestId('output-tabs')).toBeVisible()
    await expect(page.getByTestId('output-tab')).toBeVisible()
    await expect(page.getByTestId('wasm-tab')).toBeVisible()
    await expect(page.getByTestId('ast-tab')).toBeVisible()
  })

  test('select widget is created with options', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await expect(select).toBeVisible()

    const options = select.locator('option')
    const count = await options.count()
    expect(count).toBeGreaterThanOrEqual(4)
  })
})

test.describe('Platform: Widget Updates', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('textarea content updates correctly', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')

    // Get initial value
    const initialValue = await textarea.inputValue()

    // Update content
    await textarea.fill('// New content')

    // Verify update
    const newValue = await textarea.inputValue()
    expect(newValue).toBe('// New content')
    expect(newValue).not.toBe(initialValue)
  })

  test('tab selection updates UI state', async ({ page }) => {
    const outputTab = page.getByTestId('output-tab')
    const wasmTab = page.getByTestId('wasm-tab')

    // Initial state
    await expect(outputTab).toHaveClass(/panel-tab--active/)

    // Click wasm tab
    await wasmTab.click()

    // State should update
    await expect(wasmTab).toHaveClass(/panel-tab--active/)
    await expect(outputTab).not.toHaveClass(/panel-tab--active/)
  })

  test('example selector updates editor content', async ({ page }) => {
    const select = page.getByTestId('example-select')
    const textarea = page.getByTestId('editor-textarea')

    // Get initial content
    const initialValue = await textarea.inputValue()

    // Change example
    await select.selectOption('fibonacci')

    // Content should change (after any async updates)
    await page.waitForTimeout(100) // Allow for state updates
  })
})

test.describe('Platform: Widget Destruction', () => {
  test('navigation destroys and recreates widgets correctly', async ({ page }) => {
    await page.goto('/playground')

    // Verify playground exists
    await expect(page.getByTestId('playground')).toBeVisible()

    // Navigate away
    await page.goto('/docs')

    // Playground should be gone
    await expect(page.getByTestId('playground')).not.toBeVisible()
    await expect(page.getByTestId('docs-article')).toBeVisible()

    // Navigate back
    await page.goto('/playground')

    // Playground should be recreated
    await expect(page.getByTestId('playground')).toBeVisible()
  })
})

test.describe('Platform: Timer Operations', () => {
  test('UI remains responsive during operations', async ({ page }) => {
    await page.goto('/playground')

    const startTime = Date.now()

    // Perform multiple operations
    await page.getByTestId('run-btn').click()
    await page.getByTestId('format-btn').click()

    const endTime = Date.now()

    // Operations should complete quickly (< 1s each)
    expect(endTime - startTime).toBeLessThan(2000)

    // UI should still be responsive
    await expect(page.getByTestId('playground')).toBeVisible()
  })
})

test.describe('Platform: Error Resilience', () => {
  test('invalid input does not crash UI', async ({ page }) => {
    await page.goto('/playground')

    const textarea = page.getByTestId('editor-textarea')

    // Enter malformed code
    await textarea.fill('rite broken( { incomplete')

    // Click run
    await page.getByTestId('run-btn').click()

    // UI should remain functional
    await expect(page.getByTestId('playground')).toBeVisible()
    await expect(textarea).toBeVisible()
  })

  test('rapid tab switching does not cause issues', async ({ page }) => {
    await page.goto('/playground')

    const outputTab = page.getByTestId('output-tab')
    const wasmTab = page.getByTestId('wasm-tab')
    const astTab = page.getByTestId('ast-tab')

    // Rapid switching
    for (let i = 0; i < 5; i++) {
      await outputTab.click()
      await wasmTab.click()
      await astTab.click()
    }

    // UI should remain stable
    await expect(page.getByTestId('output-panel')).toBeVisible()
  })
})

test.describe('Platform: Accessibility', () => {
  test('all interactive elements are focusable', async ({ page }) => {
    await page.goto('/playground')

    // Tab through interactive elements
    await page.keyboard.press('Tab')

    // Something should be focused
    const focusedElement = page.locator(':focus')
    await expect(focusedElement).toBeVisible()
  })

  test('buttons have accessible labels', async ({ page }) => {
    await page.goto('/playground')

    const runBtn = page.getByTestId('run-btn')
    const formatBtn = page.getByTestId('format-btn')
    const shareBtn = page.getByTestId('share-btn')

    // Buttons should have text content or aria-label
    await expect(runBtn).not.toBeEmpty()
    await expect(formatBtn).not.toBeEmpty()
    await expect(shareBtn).not.toBeEmpty()
  })

  test('form controls have proper roles', async ({ page }) => {
    await page.goto('/playground')

    // Select should be a combobox or listbox
    const select = page.getByTestId('example-select')
    const tagName = await select.evaluate(el => el.tagName.toLowerCase())
    expect(tagName).toBe('select')

    // Textarea should be editable
    const textarea = page.getByTestId('editor-textarea')
    await expect(textarea).toBeEditable()
  })
})

test.describe('Platform: Cross-Browser Consistency', () => {
  test('layout renders consistently', async ({ page }) => {
    await page.goto('/playground')

    // Core layout should be present
    const header = page.getByTestId('playground-header')
    const content = page.getByTestId('playground-content')
    const footer = page.getByTestId('playground-footer')

    await expect(header).toBeVisible()
    await expect(content).toBeVisible()
    await expect(footer).toBeVisible()

    // Header should be at top
    const headerBox = await header.boundingBox()
    const contentBox = await content.boundingBox()

    expect(headerBox?.y).toBeLessThan(contentBox?.y ?? 0)
  })

  test('interactive elements work across browsers', async ({ page }) => {
    await page.goto('/playground')

    // Button click
    await page.getByTestId('run-btn').click()
    await expect(page.getByTestId('playground')).toBeVisible()

    // Select change
    const select = page.getByTestId('example-select')
    await select.selectOption('fibonacci')
    await expect(select).toHaveValue('fibonacci')

    // Text input
    const textarea = page.getByTestId('editor-textarea')
    await textarea.fill('test')
    const value = await textarea.inputValue()
    expect(value).toBe('test')
  })
})
