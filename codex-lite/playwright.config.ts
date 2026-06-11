import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/smoke',
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  fullyParallel: true,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:1420',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'desktop',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1180, height: 760 },
      },
    },
    {
      name: 'minimum-window',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 900, height: 620 },
      },
    },
    {
      name: 'narrow-webview',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 720, height: 760 },
      },
    },
  ],
  webServer: {
    command: 'TAURI_DEV_HOST=127.0.0.1 pnpm dev',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
