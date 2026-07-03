import { Page, Locator, expect } from '@playwright/test';

/**
 * Page object for the Live Results page (`/live`).
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
    await this.page.goto('/live');
    // Wait for the browse bar to render — it's the earliest stable anchor.
    await expect(this.browseLabel()).toBeVisible({ timeout: 30_000 });
  }

  // Browse bar ----------------------------------------------------------------

  browseLabel(): Locator {
    return this.page.locator('.browse-label', { hasText: /browse/i });
  }

  /**
   * Returns the loading indicator (red dot/spinner) within a specific browse button.
   * Checks for button with "Loading..." text.
   */
  browseButtonLoadingIndicator(label: string): Locator {
    return this.page.locator('button.browse-button', { hasText: 'Loading...' });
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
   * Returns the title element within a movie card.
   */
  movieCardTitle(card: Locator): Locator {
    return card.locator('.movie-title');
  }

  /**
   * Returns the description element within a movie card.
   */
  movieCardDescription(card: Locator): Locator {
    return card.locator('.movie-description');
  }

  /**
   * Returns the actors info row within a movie card.
   */
  movieCardActors(card: Locator): Locator {
    return card.locator('.movie-info-row:has-text("Cast:") .movie-info-value');
  }

  /**
   * Verifies all expected sub-components exist in a movie card.
   */
  async expectMovieCardComplete(card: Locator): Promise<void> {
    await expect(this.movieCardTitle(card)).toBeVisible();
    await expect(this.movieCardDescription(card)).toBeVisible();
    await expect(this.movieCardActors(card)).toBeVisible();
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

  // Search mode + model selector ----------------------------------------------

  modeButton(name: 'Text' | 'Hybrid' | 'Vector' | 'Structured'): Locator {
    return this.page.locator('.mode-button', { hasText: new RegExp(`^${name}$`) });
  }

  async selectMode(name: 'Text' | 'Hybrid' | 'Vector' | 'Structured'): Promise<void> {
    await this.modeButton(name).click();
  }

  modelSelect(): Locator {
    return this.page.locator('select.model-select');
  }

  /**
   * Waits for the async `/ai/models` fetch to populate the dropdown with at
   * least one model beyond the default entry. Requires mode != Text because
   * the model dropdown is hidden in Text mode.
   */
  async waitForModelsLoaded(timeout = 15_000): Promise<void> {
    await expect
      .poll(async () => this.modelSelect().locator('option').count(), { timeout })
      .toBeGreaterThan(1);
  }

  /**
   * Returns the names of every non-default model currently in the dropdown.
   */
  async availableModels(): Promise<string[]> {
    const values = await this.modelSelect().locator('option').evaluateAll(
      (opts) => (opts as HTMLOptionElement[]).map((o) => o.value),
    );
    return values.filter((v) => v !== 'default' && v !== '');
  }

  // Search bar ----------------------------------------------------------------

  searchInput(): Locator {
    return this.page.locator('input.search-input');
  }

  searchButton(): Locator {
    return this.page.locator('button.search-button');
  }

  async runSearch(query: string): Promise<void> {
    await this.searchInput().fill(query);
    await this.searchButton().click();
  }

  /**
   * Waits for the search to finish: the search button re-enables and its
   * label returns from "Searching..." to "Search".
   */
  async waitForSearchComplete(timeout = 90_000): Promise<void> {
    await expect(this.searchButton()).toHaveText('Search', { timeout });
  }
}
