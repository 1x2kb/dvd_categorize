import { Page, Locator, expect } from '@playwright/test';

/**
 * Page object for the AI Chat page (`/ai/chat`).
 *
 * The chat view requires a real Ollama backend — tests using this page
 * object belong in the `ai` project.
 */
export class ChatPage {
  readonly page: Page;

  constructor(page: Page) {
    this.page = page;
  }

  async goto(): Promise<void> {
    await this.page.goto('/ai/chat');
    await expect(this.input()).toBeVisible({ timeout: 30_000 });
  }

  input(): Locator {
    return this.page.locator('textarea.chat-input');
  }

  sendButton(): Locator {
    return this.page.locator('button.send-button');
  }

  userMessages(): Locator {
    return this.page.locator('.message-wrapper.user-message');
  }

  aiMessages(): Locator {
    return this.page.locator('.message-wrapper.ai-message');
  }

  loadingBubble(): Locator {
    return this.page.locator('.message-bubble.loading');
  }

  async send(message: string): Promise<void> {
    await this.input().fill(message);
    await this.sendButton().click();
  }

  /**
   * Waits for the AI response bubble to appear and the typing indicator to
   * clear. Tolerant: accepts any non-empty AI bubble content.
   */
  async waitForAiReply(timeout = 90_000): Promise<void> {
    // Typing indicator must disappear.
    await expect(this.loadingBubble()).toHaveCount(0, { timeout });

    // At least one AI bubble exists with non-empty text.
    await expect(this.aiMessages()).toHaveCount(1, { timeout });
    const content = this.aiMessages().first().locator('.message-content');
    await expect(content).not.toBeEmpty();
  }
}
