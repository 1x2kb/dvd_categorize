import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for the DVD categorizer E2E suite.
 *
 * Project split:
 *  - `fast`: non-AI tests. Parallel.
 *  - `ai`:   tests that hit the real Ollama service. Serial (workers: 1).
 *  - `mock`: tests that use `page.route` to mock network. Parallel.
 *
 * Prerequisites for running (see e2e/README.md):
 *  1. `docker compose up -d` at repo root.
 *  2. Desired Ollama models already pulled (visit /ai/models in the app or
 *     run `ollama pull phi3.5` against the container).
 */

const WEB_BASE_URL = process.env.E2E_WEB_URL ?? 'http://localhost:8080';
const API_BASE_URL = process.env.E2E_API_URL ?? 'http://localhost:3000';

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [
    ['list'],
    ['html', { open: 'never', outputFolder: 'playwright-report' }],
  ],

  use: {
    baseURL: WEB_BASE_URL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
  },

  projects: [
    {
      name: 'fast',
      testIgnore: [/.*\.ai\.spec\.ts/, /.*\.mock\.spec\.ts/],
      fullyParallel: true,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'ai',
      testMatch: /.*\.ai\.spec\.ts/,
      fullyParallel: false,
      workers: 1,
      retries: 2,
      timeout: 120_000,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'mock',
      testMatch: /.*\.mock\.spec\.ts/,
      fullyParallel: true,
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
