import { test, expect } from '@playwright/test';
import { LivePage } from '@pages/live.page';
import { smallestOllamaModel } from '@utils/ollama';

/**
 * AI live search — validates UI flows that depend on Ollama, without
 * iterating every available model.
 *
 * Scope:
 *   1. Default-model search in Vector mode works.
 *   2. Default-model search in Both mode works.
 *   3. Model dropdown is populated, user can change the active model, and
 *      a search with a user-chosen model completes without error.
 *
 * Each test hits real Ollama. File runs under the `ai` project
 * (workers: 1, retries: 2, timeout: 120s).
 */

test.describe.configure({ mode: 'serial' });

test.describe('AI live search', () => {
  test('Vector mode with default model returns without error', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    await live.selectMode('Vector');
    await live.runSearch('science fiction adventure');
    await live.waitForSearchComplete();

    await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);
  });

  test('Both mode with default model returns without error', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    await live.selectMode('Hybrid');
    await live.runSearch('classic comedy');
    await live.waitForSearchComplete();

    await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);
  });

  test('user can change the model via the dropdown and run a search', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    // Dropdown only appears in non-Text modes.
    await live.selectMode('Vector');
    await live.waitForModelsLoaded();

    const models = await live.availableModels();
    expect(models.length, 'at least one Ollama model must be pulled').toBeGreaterThan(0);

    // Pick the smallest pulled model to keep the test fast.
    const chosen = await smallestOllamaModel();
    await live.modelSelect().selectOption(chosen);
    await expect(live.modelSelect()).toHaveValue(chosen);

    await live.runSearch('drama');
    await live.waitForSearchComplete();

    await expect(page.getByText(/failed to load|error occurred/i)).toHaveCount(0);
  });
});
