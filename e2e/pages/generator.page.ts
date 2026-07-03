import { Page, Locator, expect } from '@playwright/test';

/**
 * Page object for the Movie Generator page (`/generator`).
 *
 * Exposes selectors for the title chip input, model dropdown, generate/clear
 * buttons, and the generated movie grid. Tests using this page object that
 * call Ollama belong in the `ai` project.
 */
export class GeneratorPage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  async goto(): Promise<void> {
    await this.page.goto('/generator');
    await expect(this.page.getByRole('heading', { name: /movie generator/i })).toBeVisible({
      timeout: 30_000,
    });
  }

  // Title input ---------------------------------------------------------------

  titleInput(): Locator {
    return this.page.locator('input.title-add-input');
  }

  addButton(): Locator {
    return this.page.locator('button', { hasText: '+ Add' });
  }

  async addTitle(title: string): Promise<void> {
    await this.titleInput().fill(title);
    await this.addButton().click();
  }

  /** Add a title by pressing Enter in the input field. */
  async addTitleViaEnter(title: string): Promise<void> {
    await this.titleInput().fill(title);
    await this.titleInput().press('Enter');
  }

  // Chip list -----------------------------------------------------------------

  chips(): Locator {
    return this.page.locator('.title-chip');
  }

  chipByTitle(title: string): Locator {
    return this.page.locator('.title-chip', { hasText: title });
  }

  async removeChip(title: string): Promise<void> {
    await this.chipByTitle(title).locator('button.title-chip-remove').click();
  }

  // Action buttons ------------------------------------------------------------

  generateButton(): Locator {
    return this.page.locator('button.button-primary', { hasText: /generate/i });
  }

  clearAllButton(): Locator {
    return this.page.locator('button.button-secondary', { hasText: /clear all/i });
  }

  // Model dropdown ------------------------------------------------------------

  modelSelect(): Locator {
    return this.page.locator('select.model-select');
  }

  async waitForModelsLoaded(timeout = 15_000): Promise<void> {
    await expect
      .poll(async () => this.modelSelect().locator('option').count(), { timeout })
      .toBeGreaterThan(1);
  }

  async availableModels(): Promise<string[]> {
    const values = await this.modelSelect().locator('option').evaluateAll(
      (opts) => (opts as HTMLOptionElement[]).map((o) => o.value),
    );
    return values.filter((v) => v !== 'default' && v !== '');
  }

  // Generated grid ------------------------------------------------------------

  movieGrid(): Locator {
    return this.page.locator('.gen-movie-grid');
  }

  movieCards(): Locator {
    return this.page.locator('.gen-movie-grid .movie-card');
  }

  lockedCards(): Locator {
    return this.page.locator('.gen-movie-grid .gen-card-locked');
  }

  skeletonCards(): Locator {
    return this.page.locator('.gen-movie-grid .gen-card-regenerating');
  }

  lockAllButton(): Locator {
    return this.page.locator('button.gen-btn-lock-all');
  }

  saveAllButton(): Locator {
    return this.page.locator('button.gen-btn-save', { hasText: /save all/i });
  }

  /**
   * Waits until all skeleton/regenerating cards have resolved into real movie
   * cards. Timeout should be generous because Ollama may be slow.
   */
  async waitForGenerationComplete(timeout = 90_000): Promise<void> {
    await expect(this.generateButton()).toHaveText('Generate', { timeout });
    await expect(this.skeletonCards()).toHaveCount(0, { timeout });
  }

  /**
   * Convenience: add a single title and click Generate.
   */
  async generateOne(title: string): Promise<void> {
    await this.addTitle(title);
    await this.generateButton().click();
  }
}
