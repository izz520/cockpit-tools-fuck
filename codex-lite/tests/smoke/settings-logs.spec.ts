import { expect, test, type Page } from '@playwright/test';

async function installSystemMock(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {
        invoke: async (command: string) => {
          if (command === 'list_codex_accounts') {
            return [];
          }
          if (command === 'get_current_codex_account') {
            return null;
          }
          if (command === 'get_system_snapshot') {
            return {
              appDataDir: '/Users/smoke/Library/Application Support/codex-lite',
              logsDir: '/Users/smoke/Library/Application Support/codex-lite/logs',
              accountsFilePath: '/Users/smoke/Library/Application Support/codex-lite/accounts.json',
              settingsFilePath: '/Users/smoke/Library/Application Support/codex-lite/settings.json',
              defaultCodexHome: '/Users/smoke/.codex',
              defaultCodexAuthFile: '/Users/smoke/.codex/auth.json',
              codexAuthFileExists: true,
            };
          }
          if (command === 'get_log_snapshot') {
            return {
              entries: [
                {
                  timestamp: '2026-06-11T00:00:00Z',
                  level: 'info',
                  message: 'Loaded auth file token=[REDACTED] api_key=[REDACTED]',
                },
              ],
            };
          }
          if (command === 'detect_codex_paths') {
            return {};
          }
          if (command === 'open_data_dir' || command === 'open_log_dir') {
            return null;
          }
          throw {
            code: 'SMOKE_UNKNOWN_COMMAND',
            message: `Unhandled smoke command: ${command}`,
            action: 'Add the command to the Playwright smoke mock.',
            retryable: false,
          };
        },
        transformCallback: () => 1,
        unregisterCallback: () => undefined,
      },
    });
  });
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(() => {
        return document.documentElement.scrollWidth <= window.innerWidth;
      }),
    )
    .toBe(true);
}

test.describe('Settings and logs smoke', () => {
  test('renders settings paths and actions', async ({ page }) => {
    await installSystemMock(page);

    await page.goto('/');
    await page.getByTitle('Settings').click();

    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible();
    await expect(page.getByText('/Users/smoke/.codex/auth.json')).toBeVisible();
    await expect(page.getByText('Found')).toBeVisible();
    await page.getByRole('button', { name: 'Detect Codex paths' }).click();
    await expectNoHorizontalOverflow(page);
  });

  test('renders redacted log snapshot', async ({ page }) => {
    await installSystemMock(page);

    await page.goto('/');
    await page.getByTitle('Logs').click();

    await expect(page.getByRole('heading', { name: 'Logs', level: 1 })).toBeVisible();
    await expect(page.getByText('Loaded auth file token=[REDACTED] api_key=[REDACTED]')).toBeVisible();
    await expect(page.getByText('sk-smoke-redacted')).toHaveCount(0);
    await expectNoHorizontalOverflow(page);
  });
});
