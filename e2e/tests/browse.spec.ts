import { test, expect } from '@playwright/test';
import { LivePage } from '@pages/live.page';

/**
 * Non-AI browse buttons on the /ai/live page.
 *
 * These buttons hit plain DB endpoints (random, recent, unknown-location)
 * and do not call Ollama. Safe to run in the `fast` project.
 *
 * Verifies results load with complete movie cards (title, description, actors).
 */
test.describe('browse bar', () => {
  const cases: Array<{ label: string; click: (p: LivePage) => Promise<void> }> = [
    { label: 'Recently Added', click: (p) => p.browseRecentlyAdded().click() },
    { label: 'Recent Releases', click: (p) => p.browseRecentReleases().click() },
    { label: 'Random Movies', click: (p) => p.browseRandom().click() },
    { label: 'Unknown Location', click: (p) => p.browseUnknownLocation().click() },
  ];

  for (const { label, click } of cases) {
    test(`clicking "${label}" loads results with complete movie card`, async ({ page }) => {
      const live = new LivePage(page);
      await live.goto();

      await click(live);

      // Wait for loading to complete (button re-enables, "Loading..." clears)
      await live.waitForIdle();

      // No unhandled error banner.
      await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);

      // Results should appear with complete movie card (title, description, actors).
      const firstCard = live.movieCards().first();
      await expect(firstCard).toBeVisible({ timeout: 10_000 });
      await live.expectMovieCardComplete(firstCard);
    });
  }
});
