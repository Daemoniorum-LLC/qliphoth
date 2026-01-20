import { test, expect } from '@playwright/test'

/**
 * Qliphoth Platform Performance Tests
 *
 * These tests measure and validate performance characteristics
 * to ensure the platform meets responsiveness requirements.
 *
 * Sprint 8: Integration Testing - Performance Benchmarks
 */

// Performance thresholds (in milliseconds)
const THRESHOLDS = {
  pageLoad: 3000,           // Max time for initial page load
  interactionResponse: 150, // Max time for button click response (includes Playwright overhead)
  typingLatency: 50,        // Max latency per keystroke
  tabSwitch: 200,           // Max time to switch tabs
  widgetCreation: 500,      // Max time to create widgets
  eventDispatch: 50,        // Max time for event dispatch
}

test.describe('Performance: Page Load', () => {
  test('initial page load is under threshold', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()

    const loadTime = Date.now() - startTime

    console.log(`Page load time: ${loadTime}ms`)
    expect(loadTime).toBeLessThan(THRESHOLDS.pageLoad)
  })

  test('navigation between pages is fast', async ({ page }) => {
    await page.goto('/')

    const startTime = Date.now()
    await page.goto('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()

    const navTime = Date.now() - startTime

    console.log(`Navigation time: ${navTime}ms`)
    expect(navTime).toBeLessThan(THRESHOLDS.pageLoad)
  })

  test('docs page loads quickly', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/docs')
    await expect(page.getByTestId('docs-article')).toBeVisible()

    const loadTime = Date.now() - startTime

    console.log(`Docs load time: ${loadTime}ms`)
    expect(loadTime).toBeLessThan(THRESHOLDS.pageLoad)
  })
})

test.describe('Performance: Interaction Response', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('button click responds quickly', async ({ page }) => {
    const runBtn = page.getByTestId('run-btn')

    const startTime = Date.now()
    await runBtn.click()
    const responseTime = Date.now() - startTime

    console.log(`Button click response: ${responseTime}ms`)
    expect(responseTime).toBeLessThan(THRESHOLDS.interactionResponse)
  })

  test('tab switch is instantaneous', async ({ page }) => {
    const wasmTab = page.getByTestId('wasm-tab')

    const startTime = Date.now()
    await wasmTab.click()
    await expect(wasmTab).toHaveClass(/panel-tab--active/)
    const switchTime = Date.now() - startTime

    console.log(`Tab switch time: ${switchTime}ms`)
    expect(switchTime).toBeLessThan(THRESHOLDS.tabSwitch)
  })

  test('select change responds quickly', async ({ page }) => {
    const select = page.getByTestId('example-select')

    const startTime = Date.now()
    await select.selectOption('fibonacci')
    await expect(select).toHaveValue('fibonacci')
    const changeTime = Date.now() - startTime

    console.log(`Select change time: ${changeTime}ms`)
    expect(changeTime).toBeLessThan(THRESHOLDS.interactionResponse)
  })
})

test.describe('Performance: Typing', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('typing has low latency', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()
    await textarea.fill('')

    const testString = 'performance test'
    const startTime = Date.now()

    await textarea.fill(testString)

    const totalTime = Date.now() - startTime
    const perCharTime = totalTime / testString.length

    console.log(`Total typing time: ${totalTime}ms, per char: ${perCharTime.toFixed(2)}ms`)
    expect(perCharTime).toBeLessThan(THRESHOLDS.typingLatency)
  })

  test('long text input remains responsive', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()

    // Generate long text
    const longText = 'rite test() { }\n'.repeat(50)

    const startTime = Date.now()
    await textarea.fill(longText)
    const fillTime = Date.now() - startTime

    console.log(`Long text fill time: ${fillTime}ms`)

    // Should still be responsive
    await expect(textarea).toBeEditable()
  })
})

test.describe('Performance: Widget Operations', () => {
  test('playground widgets load quickly', async ({ page }) => {
    const startTime = Date.now()

    await page.goto('/playground')

    // Wait for all key widgets
    await Promise.all([
      expect(page.getByTestId('playground-header')).toBeVisible(),
      expect(page.getByTestId('editor-panel')).toBeVisible(),
      expect(page.getByTestId('output-panel')).toBeVisible(),
      expect(page.getByTestId('playground-footer')).toBeVisible(),
    ])

    const loadTime = Date.now() - startTime

    console.log(`Widget creation time: ${loadTime}ms`)
    expect(loadTime).toBeLessThan(THRESHOLDS.widgetCreation)
  })

  test('dynamic content updates quickly', async ({ page }) => {
    await page.goto('/playground')

    const textarea = page.getByTestId('editor-textarea')

    // Measure content update time
    const startTime = Date.now()
    await textarea.fill('// Updated content')
    const updateTime = Date.now() - startTime

    console.log(`Content update time: ${updateTime}ms`)
    expect(updateTime).toBeLessThan(THRESHOLDS.interactionResponse)
  })
})

test.describe('Performance: Event Dispatch', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('rapid events are handled efficiently', async ({ page }) => {
    const runBtn = page.getByTestId('run-btn')

    const eventCount = 20
    const startTime = Date.now()

    // Fire many events rapidly
    for (let i = 0; i < eventCount; i++) {
      await runBtn.click({ force: true })
    }

    const totalTime = Date.now() - startTime
    const perEventTime = totalTime / eventCount

    console.log(`Total event time: ${totalTime}ms, per event: ${perEventTime.toFixed(2)}ms`)
    expect(perEventTime).toBeLessThan(THRESHOLDS.eventDispatch)
  })

  test('keyboard events are processed quickly', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()

    const startTime = Date.now()

    // Rapid keyboard input
    for (let i = 0; i < 20; i++) {
      await page.keyboard.press('a')
    }

    const totalTime = Date.now() - startTime

    console.log(`Keyboard event time: ${totalTime}ms`)
    expect(totalTime).toBeLessThan(1000) // 20 events in 1 second
  })
})

test.describe('Performance: Memory & Stability', () => {
  test('repeated navigation does not leak', async ({ page }) => {
    // Navigate multiple times
    for (let i = 0; i < 5; i++) {
      await page.goto('/playground')
      await expect(page.getByTestId('playground')).toBeVisible()

      await page.goto('/docs')
      await expect(page.getByTestId('docs-article')).toBeVisible()
    }

    // Final navigation should still be fast
    const startTime = Date.now()
    await page.goto('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()
    const loadTime = Date.now() - startTime

    console.log(`Load after repeated nav: ${loadTime}ms`)
    expect(loadTime).toBeLessThan(THRESHOLDS.pageLoad)
  })

  test('long session remains responsive', async ({ page }) => {
    await page.goto('/playground')

    // Simulate extended use
    for (let i = 0; i < 10; i++) {
      await page.getByTestId('run-btn').click()
      await page.getByTestId('format-btn').click()
      await page.getByTestId('output-tab').click()
      await page.getByTestId('wasm-tab').click()
    }

    // Should still be responsive
    const startTime = Date.now()
    await page.getByTestId('run-btn').click()
    const responseTime = Date.now() - startTime

    console.log(`Response after extended use: ${responseTime}ms`)
    expect(responseTime).toBeLessThan(THRESHOLDS.interactionResponse * 2)
  })
})

test.describe('Performance: Rendering', () => {
  test('initial render is complete', async ({ page }) => {
    await page.goto('/playground')

    // All visual elements should be rendered
    const elements = [
      page.getByTestId('playground-header'),
      page.getByTestId('editor-panel'),
      page.getByTestId('output-panel'),
      page.getByTestId('run-btn'),
      page.getByTestId('example-select'),
    ]

    for (const element of elements) {
      await expect(element).toBeVisible()
    }
  })

  test('responsive resize is smooth', async ({ page }) => {
    await page.goto('/playground')

    // Resize viewport
    await page.setViewportSize({ width: 1280, height: 800 })
    await expect(page.getByTestId('playground')).toBeVisible()

    await page.setViewportSize({ width: 768, height: 1024 })
    await expect(page.getByTestId('playground')).toBeVisible()

    await page.setViewportSize({ width: 375, height: 667 })
    await expect(page.getByTestId('playground')).toBeVisible()

    // Should still be responsive
    await page.getByTestId('run-btn').click()
    await expect(page.getByTestId('playground')).toBeVisible()
  })
})

test.describe('Performance: Metrics Collection', () => {
  test('collects core web vitals', async ({ page }) => {
    // Navigate and wait for load
    await page.goto('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()

    // Get performance metrics
    const metrics = await page.evaluate(() => {
      const entries = performance.getEntriesByType('navigation') as PerformanceNavigationTiming[]
      const nav = entries[0]

      return {
        domContentLoaded: nav.domContentLoadedEventEnd - nav.startTime,
        loadComplete: nav.loadEventEnd - nav.startTime,
        firstPaint: performance.getEntriesByName('first-paint')[0]?.startTime ?? 0,
        firstContentfulPaint: performance.getEntriesByName('first-contentful-paint')[0]?.startTime ?? 0,
      }
    })

    console.log('Performance Metrics:')
    console.log(`  DOM Content Loaded: ${metrics.domContentLoaded.toFixed(2)}ms`)
    console.log(`  Load Complete: ${metrics.loadComplete.toFixed(2)}ms`)
    console.log(`  First Paint: ${metrics.firstPaint.toFixed(2)}ms`)
    console.log(`  First Contentful Paint: ${metrics.firstContentfulPaint.toFixed(2)}ms`)

    // Verify reasonable performance
    expect(metrics.firstContentfulPaint).toBeLessThan(2000)
  })
})
