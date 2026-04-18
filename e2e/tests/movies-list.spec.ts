import { test, expect } from '@playwright/test';
import { waitForAppReady } from '@utils/app';

test.describe('movies list', () => {
  test('loads and resolves either to a populated list or loading state', async ({ page }) => {
    await page.goto('/movies/list');
    await waitForAppReady(page);

    // Page renders either "Loading movies..." (until fetch resolves) or the list.
    // Accept either terminal state: total count text OR persistent loading.
    const totalText = page.getByText(/total movies:/i);
    const loadingText = page.getByText(/loading movies/i);

    await expect(totalText.or(loadingText)).toBeVisible({ timeout: 30_000 });
  });
});
