import { test, expect } from '@playwright/test';
import { GeneratorPage } from '@pages/generator.page';

/**
 * AI Movie Generator — end-to-end tests that hit the real Ollama service.
 *
 * Scope:
 *   1. Page renders its input controls.
 *   2. Adding a title creates a chip; Clear All removes chips and grid.
 *   3. Generate produces at least one completed movie card (title + description).
 *   4. The Lock All / Unlock All toolbar appears after generation and toggles correctly.
 *   5. A locked card is skipped on re-generate; an unlocked card is regenerated.
 *
 * Runs under the `ai` project (workers: 1, retries: 2, timeout: 120 s).
 *
 * IMPORTANT: These tests must never insert movies into the catalog.
 * A route intercept on POST /saveMovie enforces this — any accidental save
 * call will abort the request and fail the test immediately.
 */

test.describe.configure({ mode: 'serial' });

test.describe('AI generator', () => {
  test.beforeEach(async ({ page }) => {
    // Block any attempt to save a movie to the catalog. If Save or Save All is
    // ever clicked during these tests the request is aborted and the test fails.
    await page.route('**/saveMovie', (route) => {
      route.abort();
      throw new Error('Test must not save movies to the catalog — /saveMovie was called');
    });
  });

  test('page renders heading, input, and action buttons', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await expect(gen.titleInput()).toBeVisible();
    await expect(gen.addButton()).toBeVisible();
    await expect(gen.generateButton()).toBeVisible();
    await expect(gen.clearAllButton()).toBeVisible();
    await expect(gen.modelSelect()).toBeVisible();
  });

  test('adding a title creates a chip; Clear All removes everything', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.addTitle('Inception');
    await expect(gen.chips()).toHaveCount(1);
    await expect(gen.chipByTitle('Inception')).toBeVisible();

    await gen.clearAllButton().click();
    await expect(gen.chips()).toHaveCount(0);
    await expect(gen.movieGrid()).toHaveCount(0);
  });

  test('pipe separator creates multiple chips at once', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.titleInput().fill('The Matrix | Jurassic Park');
    await gen.addButton().click();

    await expect(gen.chips()).toHaveCount(2);
    await expect(gen.chipByTitle('The Matrix')).toBeVisible();
    await expect(gen.chipByTitle('Jurassic Park')).toBeVisible();
  });

  test('generate produces a completed movie card with title and description', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.generateOne('Metropolis');
    await gen.waitForGenerationComplete();

    const cards = gen.movieCards();
    await expect(cards).toHaveCount(1);

    const card = cards.first();
    await expect(card.locator('.movie-title')).toBeVisible();
    await expect(card.locator('.movie-description')).not.toBeEmpty();
  });

  test('movie already in catalog shows badge and no lock button', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.generateOne('Inception');
    await gen.waitForGenerationComplete();

    const card = gen.movieCards().first();
    await expect(card.locator('.gen-catalog-badge')).toBeVisible();
    await expect(card.locator('.gen-catalog-badge')).toHaveText(/already in catalog/i);
    // Lock button must not be rendered for catalog items.
    await expect(card.locator('button.gen-btn-lock')).toHaveCount(0);
  });

  test('Lock All button appears after generation and toggles lock state', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.generateOne('Metropolis');
    await gen.waitForGenerationComplete();

    const lockAll = gen.lockAllButton();
    await expect(lockAll).toBeVisible();
    await expect(lockAll).toHaveText(/lock all/i);

    // Lock all cards.
    await lockAll.click();
    await expect(lockAll).toHaveText(/unlock all/i);
    await expect(gen.movieCards().first()).toHaveAttribute('class', /gen-card-locked/);

    // Unlock all cards.
    await lockAll.click();
    await expect(lockAll).toHaveText(/lock all/i);
    await expect(gen.movieCards().first()).not.toHaveAttribute('class', /gen-card-locked/);
  });

  test('locked card is preserved on re-generate', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await gen.titleInput().fill('Metropolis | Nosferatu');
    await gen.addButton().click();
    await gen.generateButton().click();
    await gen.waitForGenerationComplete();

    // Lock the first card and capture its description.
    const descBefore = await gen.movieCards().first().locator('.movie-description').textContent();
    await gen.movieCards().first().locator('button.gen-btn-lock').click();
    await expect(gen.movieCards().first()).toHaveAttribute('class', /gen-card-locked/);

    // Re-generate — locked card must not change.
    await gen.generateButton().click();
    await gen.waitForGenerationComplete();

    const descAfter = await gen.movieCards().first().locator('.movie-description').textContent();
    expect(descAfter).toBe(descBefore);
  });

});
