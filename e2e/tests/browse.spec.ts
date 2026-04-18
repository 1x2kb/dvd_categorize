import { test, expect } from '@playwright/test';
import { LivePage } from '@pages/live.page';

/**
 * Non-AI browse buttons on the /ai/live page.
 *
 * These buttons hit plain DB endpoints (random, recent, unknown-location)
 * and do not call Ollama. Safe to run in the `fast` project.
 *
 * Tolerant assertions: we require the loading state to clear and no page
 * error, but do not require a specific number of results. The dev DB may
 * legitimately have zero movies at some location filters.
 */
test.describe('browse bar', () => {
  const cases: Array<{ label: string; click: (p: LivePage) => Promise<void> }> = [
    { label: 'Recently Added', click: (p) => p.browseRecentlyAdded().click() },
    { label: 'Recent Releases', click: (p) => p.browseRecentReleases().click() },
    { label: 'Random Movies', click: (p) => p.browseRandom().click() },
    { label: 'Unknown Location', click: (p) => p.browseUnknownLocation().click() },
  ];

  for (const { label, click } of cases) {
    test(`clicking "${label}" completes without error`, async ({ page }) => {
      const live = new LivePage(page);
      await live.goto();

      await click(live);
      await live.waitForIdle();

      // No unhandled error banner.
      await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);
    });
  }
});
