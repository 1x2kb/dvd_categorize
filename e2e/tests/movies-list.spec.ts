import { test, expect } from '@playwright/test';
import { waitForAppReady } from '@utils/app';

test.describe('movies list', () => {
  test('live page loads and renders the browse bar', async ({ page }) => {
    await page.goto('/live');
    await waitForAppReady(page);

    // Browse bar should be visible once the app has booted.
    await expect(page.locator('.browse-label', { hasText: /browse/i })).toBeVisible({
      timeout: 30_000,
    });
  });
});
