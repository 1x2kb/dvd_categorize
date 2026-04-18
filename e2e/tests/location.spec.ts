import { test, expect } from '@playwright/test';
import { LivePage } from '@pages/live.page';

test.describe('location browse', () => {
  test('location dropdown is populated from the API', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    await expect(live.locationSelect()).toBeVisible();
    await live.waitForLocationsLoaded();
  });

  test('selecting a location and clicking the button completes without error', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    await live.waitForLocationsLoaded();

    // First non-placeholder option.
    const select = live.locationSelect();
    const firstLocation =
      (await select.locator('option').nth(1).getAttribute('value')) ?? '';
    expect(firstLocation).not.toBe('');

    await select.selectOption(firstLocation);
    await live.moviesByLocationButton().click();
    await live.waitForIdle();

    await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);
  });
});
