import { Page, Locator, expect } from '@playwright/test';

/**
 * Page object for the AI Live Results page (`/ai/live`).
 *
 * Exposes selectors for the non-AI browse bar, location dropdown, and
 * the shared movie grid. AI-specific selectors (model dropdown, search
 * enhancement toggle) belong to a separate AI page object.
 */
export class LivePage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  async goto(): Promise<void> {
    await this.page.goto('/ai/live');
    // Wait for the browse bar to render — it's the earliest stable anchor.
    await expect(this.browseLabel()).toBeVisible({ timeout: 30_000 });
  }

  // Browse bar ----------------------------------------------------------------

  browseLabel(): Locator {
    return this.page.locator('.browse-label', { hasText: /browse/i });
  }

  browseRecentlyAdded(): Locator {
    return this.page.getByRole('button', { name: 'Recently Added' });
  }

  browseRecentReleases(): Locator {
    return this.page.getByRole('button', { name: 'Recent Releases' });
  }

  browseRandom(): Locator {
    return this.page.getByRole('button', { name: 'Random Movies' });
  }

  browseUnknownLocation(): Locator {
    return this.page.getByRole('button', { name: 'Unknown Location' });
  }

  // Location browse group -----------------------------------------------------

  locationSelect(): Locator {
    return this.page.locator('select.location-select');
  }

  /**
   * Waits for the async `/uniqueLocations()` fetch to populate the dropdown.
   * Resolves once at least one real option (beyond the "Select location..."
   * placeholder) is present. Throws on timeout.
   */
  async waitForLocationsLoaded(timeout = 15_000): Promise<void> {
    await expect
      .poll(async () => this.locationSelect().locator('option').count(), { timeout })
      .toBeGreaterThan(1);
  }

  moviesByLocationButton(): Locator {
    return this.page.getByRole('button', { name: 'Movies by Location' });
  }

  // Movie grid ----------------------------------------------------------------

  movieCards(): Locator {
    return this.page.locator('.movie-card');
  }

  /**
   * Waits for any in-flight browse/search request to settle. The BrowseBar
   * re-enables its buttons when loading ends, so we watch for that.
   */
  async waitForIdle(): Promise<void> {
    await expect(
      this.page.locator('.browse-button', { hasText: 'Loading...' }),
    ).toHaveCount(0, { timeout: 30_000 });
  }
}
