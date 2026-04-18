import { test, expect } from '@playwright/test';
import { ChatPage } from '@pages/chat.page';

/**
 * AI chat — single-message round trip against real Ollama.
 *
 * The chat view currently hardcodes `phi3.5` as its model (no UI dropdown),
 * so only the default flow is tested. When a model selector lands on this
 * view, mirror the ai-search pattern and add a "change model via UI" test.
 */

test.describe.configure({ mode: 'serial' });

test.describe('AI chat', () => {
  test('chat view renders input and send controls', async ({ page }) => {
    const chat = new ChatPage(page);
    await chat.goto();

    await expect(chat.input()).toBeVisible();
    await expect(chat.sendButton()).toBeVisible();
    await expect(page.getByRole('heading', { name: /movie collection chat/i })).toBeVisible();
  });

  test('sending a message produces an AI reply bubble', async ({ page }) => {
    const chat = new ChatPage(page);
    await chat.goto();

    await chat.send('Hello');

    // User bubble lands immediately (client-side echo before network).
    await expect(chat.userMessages()).toHaveCount(1, { timeout: 5_000 });

    // AI bubble follows after Ollama responds.
    await chat.waitForAiReply();
  });
});
