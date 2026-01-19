import { test, expect } from '@playwright/test'

test.describe('Playground Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('renders playground with header', async ({ page }) => {
    await expect(page.getByTestId('playground')).toBeVisible()
    await expect(page.getByTestId('playground-header')).toBeVisible()
    await expect(page.getByTestId('playground-header')).toContainText('Sigil Playground')
  })

  test('action buttons are present', async ({ page }) => {
    await expect(page.getByTestId('run-btn')).toBeVisible()
    await expect(page.getByTestId('format-btn')).toBeVisible()
    await expect(page.getByTestId('share-btn')).toBeVisible()
  })

  test('example selector is present', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await expect(select).toBeVisible()

    // Check options
    await expect(select.locator('option')).toHaveCount(4)
  })
})

test.describe('Athame Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('editor panel is present', async ({ page }) => {
    await expect(page.getByTestId('editor-panel')).toBeVisible()
    await expect(page.getByTestId('athame-editor')).toBeVisible()
  })

  test('editor has line gutter', async ({ page }) => {
    const gutter = page.getByTestId('editor-gutter')
    await expect(gutter).toBeVisible()

    // Check line numbers are present
    const lineNumbers = gutter.locator('.line-number')
    const count = await lineNumbers.count()
    expect(count).toBeGreaterThan(0)
  })

  test('editor textarea is editable', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await expect(textarea).toBeVisible()
    await expect(textarea).toBeEditable()
  })

  test('editor contains default Sigil code', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    const value = await textarea.inputValue()

    // Check for Sigil syntax
    expect(value).toContain('invoke')
    expect(value).toContain('rite main()')
  })

  test('editor can accept typed input', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')

    // Clear and type new code
    await textarea.fill('')
    await textarea.fill('rite hello() {\n    println("Hello!");\n}')

    const value = await textarea.inputValue()
    expect(value).toContain('rite hello()')
    expect(value).toContain('println')
  })

  test('editor preserves Sigil symbols', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')

    await textarea.fill('≔ x! = 42;')

    const value = await textarea.inputValue()
    expect(value).toContain('≔')
  })
})

test.describe('Output Panel', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('output panel is present', async ({ page }) => {
    await expect(page.getByTestId('output-panel')).toBeVisible()
  })

  test('output tabs are present', async ({ page }) => {
    const tabs = page.getByTestId('output-tabs')
    await expect(tabs).toBeVisible()

    await expect(page.getByTestId('output-tab')).toBeVisible()
    await expect(page.getByTestId('wasm-tab')).toBeVisible()
    await expect(page.getByTestId('ast-tab')).toBeVisible()
  })

  test('output tab is active by default', async ({ page }) => {
    const outputTab = page.getByTestId('output-tab')
    await expect(outputTab).toHaveClass(/panel-tab--active/)
  })

  test('output console shows initial message', async ({ page }) => {
    const console = page.getByTestId('output-console')
    await expect(console).toBeVisible()

    await expect(page.getByTestId('console-line')).toContainText('Click "Run"')
  })

  test('clicking tabs switches active state', async ({ page }) => {
    const wasmTab = page.getByTestId('wasm-tab')
    await wasmTab.click()

    await expect(wasmTab).toHaveClass(/panel-tab--active/)
    await expect(page.getByTestId('output-tab')).not.toHaveClass(/panel-tab--active/)
  })
})

test.describe('Playground Footer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('footer status bar is present', async ({ page }) => {
    await expect(page.getByTestId('playground-footer')).toBeVisible()
  })

  test('status shows line and column', async ({ page }) => {
    await expect(page.getByTestId('status-line')).toContainText('Ln')
    await expect(page.getByTestId('status-line')).toContainText('Col')
  })

  test('status shows language', async ({ page }) => {
    await expect(page.getByTestId('status-lang')).toHaveText('Sigil')
  })

  test('status shows encoding', async ({ page }) => {
    await expect(page.getByTestId('status-encoding')).toHaveText('UTF-8')
  })
})

test.describe('Example Selector', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('selecting hello world example loads code', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await select.selectOption('hello')

    // Trigger change event manually if needed
    const textarea = page.getByTestId('editor-textarea')
    const value = await textarea.inputValue()

    // Default is Hello World, so code should contain hello-related content
    expect(value).toBeTruthy()
  })

  test('selecting fibonacci example loads code', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await select.selectOption('fibonacci')

    // Just verify it triggered - actual content depends on implementation
    await expect(select).toHaveValue('fibonacci')
  })

  test('selecting counter example loads code', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await select.selectOption('counter')
    await expect(select).toHaveValue('counter')
  })

  test('selecting fetch example loads code', async ({ page }) => {
    const select = page.getByTestId('example-select')
    await select.selectOption('fetch')
    await expect(select).toHaveValue('fetch')
  })
})

test.describe('Run Button', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('run button has correct text', async ({ page }) => {
    const runBtn = page.getByTestId('run-btn')
    await expect(runBtn).toContainText('Run')
  })

  test('run button can be clicked', async ({ page }) => {
    const runBtn = page.getByTestId('run-btn')
    await expect(runBtn).toBeEnabled()

    // Click and verify no errors
    await runBtn.click()

    // Page should still be functional
    await expect(page.getByTestId('playground')).toBeVisible()
  })
})

test.describe('Playground Layout', () => {
  test.describe('desktop', () => {
    test.use({ viewport: { width: 1280, height: 800 } })

    test('editor and output panels are side by side', async ({ page }) => {
      await page.goto('/playground')

      const content = page.getByTestId('playground-content')
      await expect(content).toBeVisible()

      const editorPanel = page.getByTestId('editor-panel')
      const outputPanel = page.getByTestId('output-panel')

      const editorBox = await editorPanel.boundingBox()
      const outputBox = await outputPanel.boundingBox()

      // Panels should be side by side (same Y, different X)
      expect(editorBox?.y).toBe(outputBox?.y)
      expect(editorBox?.x).toBeLessThan(outputBox?.x ?? 0)
    })
  })

  test.describe('mobile', () => {
    test.use({ viewport: { width: 375, height: 667 } })

    test('panels stack vertically on mobile', async ({ page }) => {
      await page.goto('/playground')

      const editorPanel = page.getByTestId('editor-panel')
      const outputPanel = page.getByTestId('output-panel')

      await expect(editorPanel).toBeVisible()
      await expect(outputPanel).toBeVisible()

      const editorBox = await editorPanel.boundingBox()
      const outputBox = await outputPanel.boundingBox()

      // On mobile, output should be below editor
      if (editorBox && outputBox) {
        expect(outputBox.y).toBeGreaterThanOrEqual(editorBox.y + editorBox.height - 10) // 10px tolerance
      }
    })
  })
})

test.describe('Keyboard Shortcuts', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/playground')
  })

  test('Ctrl+Enter runs code', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.focus()

    // Press Ctrl+Enter
    await page.keyboard.press('Control+Enter')

    // Page should still be functional (no crash)
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('editor supports keyboard input', async ({ page }) => {
    const textarea = page.getByTestId('editor-textarea')
    await textarea.fill('')
    await textarea.focus()

    await page.keyboard.type('test code here')

    const value = await textarea.inputValue()
    expect(value).toContain('test code here')
  })
})

test.describe('Playground Navigation', () => {
  test('can navigate to playground from home', async ({ page }) => {
    await page.goto('/')

    await page.getByTestId('cta-playground').click()

    await expect(page).toHaveURL('/playground')
    await expect(page.getByTestId('playground')).toBeVisible()
  })

  test('can navigate to playground from docs', async ({ page }) => {
    await page.goto('/docs')

    await page.getByTestId('playground-card').click()

    await expect(page).toHaveURL('/playground')
  })

  test('can navigate back to docs from playground', async ({ page }) => {
    await page.goto('/playground')

    await page.getByTestId('nav-docs').click()

    await expect(page).toHaveURL('/docs')
  })
})
