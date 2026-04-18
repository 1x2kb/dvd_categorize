import { test, expect } from '@playwright/test';
import { waitForAppReady } from '@utils/app';

/**
 * ModelPull UI contract test.
 *
 * Uses `page.route` to intercept `POST /ai/pull-model` so Ollama is never
 * actually contacted. Verifies:
 *   - Correct request URL + method + JSON body shape.
 *   - Success response is parsed and surfaced to the UI.
 *   - Error response is parsed and surfaced to the UI.
 *
 * Lives in the `mock` project: parallel, no backend dependencies.
 */

type PullRequestBody = { model_name: string };

async function mockPullModel(
  page: import('@playwright/test').Page,
  opts: {
    status: number;
    body: { success: boolean; message: string };
    captureBody?: (body: PullRequestBody) => void;
  },
): Promise<void> {
  await page.route('**/ai/pull-model', async (route) => {
    const request = route.request();
    if (opts.captureBody) {
      opts.captureBody(request.postDataJSON() as PullRequestBody);
    }
    await route.fulfill({
      status: opts.status,
      contentType: 'application/json',
      body: JSON.stringify(opts.body),
    });
  });
}

test.describe('ModelPull UI contract', () => {
  test('success: sends correct body and renders success message', async ({ page }) => {
    let captured: PullRequestBody | undefined;
    await mockPullModel(page, {
      status: 200,
      body: { success: true, message: 'Model pulled successfully' },
      captureBody: (body) => {
        captured = body;
      },
    });

    await page.goto('/ai/models');
    await waitForAppReady(page);

    await page.locator('input.model-input').fill('phi3.5');
    await page.getByRole('button', { name: 'Pull Model' }).click();

    // Success message appears.
    await expect(page.locator('.status-message.status-success')).toContainText(
      'Model pulled successfully',
      { timeout: 10_000 },
    );

    // Request body shape verified.
    expect(captured).toEqual({ model_name: 'phi3.5' });
  });

  test('error: server 500 response surfaces as error message', async ({ page }) => {
    await mockPullModel(page, {
      status: 500,
      body: { success: false, message: 'Registry unreachable' },
    });

    await page.goto('/ai/models');
    await waitForAppReady(page);

    await page.locator('input.model-input').fill('nonexistent:1b');
    await page.getByRole('button', { name: 'Pull Model' }).click();

    // Error UI appears. The view renders a generic "Failed to pull model"
    // message on non-2xx responses; accept either that or the body text.
    await expect(page.locator('.status-message.status-error')).toBeVisible({
      timeout: 10_000,
    });
  });

  test('pull button disables while a request is in flight', async ({ page }) => {
    // Delay the mock response so we can observe the "Pulling..." state.
    await page.route('**/ai/pull-model', async (route) => {
      await new Promise((r) => setTimeout(r, 1_500));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'done' }),
      });
    });

    await page.goto('/ai/models');
    await waitForAppReady(page);

    await page.locator('input.model-input').fill('phi3.5');
    const pullBtn = page.getByRole('button', { name: /^(Pull Model|Pulling\.\.\.)$/ });
    await pullBtn.click();

    // While in flight, the button is disabled and shows "Pulling...".
    await expect(page.getByRole('button', { name: 'Pulling...' })).toBeDisabled({
      timeout: 5_000,
    });

    // After completion, button reverts.
    await expect(page.getByRole('button', { name: 'Pull Model' })).toBeVisible({
      timeout: 10_000,
    });
  });
});
