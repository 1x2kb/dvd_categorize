import { test, expect } from '@playwright/test';
import { LivePage } from '@pages/live.page';
import { GeneratorPage } from '@pages/generator.page';

/**
 * Model dropdown UI tests — verify the select element is present and that
 * choosing a different option updates its value. No generation is triggered.
 *
 * Runs under the `fast` project (no Ollama required).
 */

test.describe('model select — live search', () => {
  test('selecting different options updates the dropdown value', async ({ page }) => {
    const live = new LivePage(page);
    await live.goto();

    // Dropdown is hidden in Text mode — switch to Vector to reveal it.
    await live.selectMode('Vector');
    await expect(live.modelSelect()).toBeVisible();

    await live.waitForModelsLoaded();
    const models = await live.availableModels();
    expect(models.length, 'at least two models must be available').toBeGreaterThan(1);

    await live.modelSelect().selectOption(models[0]);
    await expect(live.modelSelect()).toHaveValue(models[0]);

    await live.modelSelect().selectOption(models[1]);
    await expect(live.modelSelect()).toHaveValue(models[1]);
  });
});

test.describe('model select — generator', () => {
  test('selecting different options updates the dropdown value', async ({ page }) => {
    const gen = new GeneratorPage(page);
    await gen.goto();

    await expect(gen.modelSelect()).toBeVisible();

    await gen.waitForModelsLoaded();
    const models = await gen.availableModels();
    expect(models.length, 'at least two models must be available').toBeGreaterThan(1);

    await gen.modelSelect().selectOption(models[0]);
    await expect(gen.modelSelect()).toHaveValue(models[0]);

    await gen.modelSelect().selectOption(models[1]);
    await expect(gen.modelSelect()).toHaveValue(models[1]);
  });
});
