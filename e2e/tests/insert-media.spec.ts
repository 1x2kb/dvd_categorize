import { test, expect } from '@playwright/test';
import { waitForAppReady } from '@utils/app';

/**
 * Insert Media page (`/moives/new` — yes, the route has a typo).
 *
 * Scope for M2: only verify the static UI renders and is interactive.
 * Actual CSV submission is deferred until we have a safe strategy
 * (uniquely-tagged test rows with cleanup).
 */
test.describe('insert media', () => {
  test('page renders CSV form controls', async ({ page }) => {
    await page.goto('/moives/new');
    await waitForAppReady(page);

    // Instructions block.
    await expect(page.getByText(/csv format/i)).toBeVisible();

    // Textarea accepts input.
    const textarea = page.locator('textarea.csv-textarea');
    await expect(textarea).toBeVisible();
    await textarea.fill('Title,Year\n"Test",1999');
    await expect(textarea).toHaveValue(/Test/);

    // Both action buttons present.
    await expect(page.getByRole('button', { name: /export all movies/i })).toBeVisible();
  });
});
