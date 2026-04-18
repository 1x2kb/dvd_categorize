import { test, expect } from '@playwright/test';
import { waitForAppReady } from '@utils/app';

test.describe('smoke', () => {
  test('home page loads and app shell renders', async ({ page }) => {
    await page.goto('/');
    await waitForAppReady(page);

    // Primary nav is present.
    await expect(page.getByRole('link', { name: /insert/i }).first()).toBeVisible();
    await expect(page.getByRole('link', { name: /models/i }).first()).toBeVisible();

    // No visible error banner.
    await expect(page.getByText(/failed to load|error/i)).toHaveCount(0);
  });

  test('API is reachable', async ({ request }) => {
    const apiBase = process.env.E2E_API_URL ?? 'http://localhost:3000';
    const res = await request.get(`${apiBase}/ai/models`);
    expect(res.status(), 'GET /ai/models should succeed').toBeLessThan(500);
  });
});
