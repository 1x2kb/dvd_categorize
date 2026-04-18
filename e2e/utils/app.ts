import { Page, expect } from '@playwright/test';

/**
 * Waits for the Dioxus WASM app shell to be interactive.
 *
 * Strategy: wait for the primary nav to render. The app's root layout
 * renders nav links unconditionally once WASM has booted.
 */
export async function waitForAppReady(page: Page): Promise<void> {
  await expect(
    page.getByRole('link', { name: /insert/i }).first(),
  ).toBeVisible({ timeout: 30_000 });
}

/**
 * Asserts the browser console produced no errors while `fn` executed.
 */
export async function expectNoConsoleErrors(
  page: Page,
  fn: () => Promise<void>,
): Promise<void> {
  const errors: string[] = [];
  const listener = (msg: import('@playwright/test').ConsoleMessage) => {
    if (msg.type() === 'error') {
      errors.push(msg.text());
    }
  };
  page.on('console', listener);
  try {
    await fn();
  } finally {
    page.off('console', listener);
  }
  expect(errors, `Console errors detected:\n${errors.join('\n')}`).toHaveLength(0);
}
