import { test, expect } from '@playwright/test';

test('WASM module loads and executes', async ({ page }) => {
    // Collect console logs
    const logs: string[] = [];
    page.on('console', msg => {
        logs.push(`[${msg.type()}] ${msg.text()}`);
    });

    // Collect errors
    const errors: string[] = [];
    page.on('pageerror', err => {
        errors.push(err.message);
    });

    // Navigate to test page
    await page.goto('http://localhost:5180/runtime/test.html');

    // Wait for either success or error
    await page.waitForFunction(() => {
        const status = document.getElementById('status');
        return status?.classList.contains('success') || status?.classList.contains('error');
    }, { timeout: 10000 });

    // Check status
    const statusText = await page.locator('#status').textContent();
    const outputText = await page.locator('#output').textContent();

    console.log('=== Console Logs ===');
    logs.forEach(log => console.log(log));

    console.log('\n=== Page Errors ===');
    errors.forEach(err => console.log(err));

    console.log('\n=== Status ===');
    console.log(statusText);

    console.log('\n=== Output ===');
    console.log(outputText);

    // If there were errors, fail with details
    if (errors.length > 0) {
        throw new Error(`WASM errors: ${errors.join('\n')}`);
    }

    // Check for success
    const hasSuccess = await page.locator('#status.success').count();
    expect(hasSuccess).toBe(1);
});
