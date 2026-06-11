import { expect, test, type Page } from '@playwright/test';

interface TauriMockOptions {
  accounts?: unknown[];
  currentAccount?: unknown | null;
}

const oauthAccount = {
  id: 'oauth-long-email',
  displayName: 'Work Codex OAuth',
  email: 'very.long.codex.user.name.for.layout.validation@example-enterprise-domain.test',
  authMode: 'oauth',
  accountId: 'acct_team_codex_lite_smoke_long_identifier_001',
  userId: 'user_smoke_001',
  planType: 'pro',
  apiBaseUrl: null,
  quota: {
    hourlyRemainingPercent: 82,
    hourlyResetAt: 1_800_000_000,
    weeklyRemainingPercent: 64,
    weeklyResetAt: 1_800_086_400,
    updatedAt: 1_799_990_000,
    stale: false,
  },
  quotaError: null,
  tags: ['smoke'],
  note: null,
  createdAt: 1_799_900_000,
  updatedAt: 1_799_990_000,
  lastUsedAt: 1_799_991_000,
  isCurrent: true,
  capabilityWarning: null,
};

const apiKeyAccount = {
  id: 'api-key-smoke',
  displayName: 'API Key Only',
  email: null,
  authMode: 'api_key',
  accountId: null,
  userId: null,
  planType: null,
  apiBaseUrl: 'https://api.openai.com/v1',
  quota: null,
  quotaError: null,
  tags: ['api'],
  note: null,
  createdAt: 1_799_900_100,
  updatedAt: 1_799_900_100,
  lastUsedAt: null,
  isCurrent: false,
  capabilityWarning: 'API key accounts cannot refresh ChatGPT quota.',
};

async function installTauriMock(page: Page, options: TauriMockOptions): Promise<void> {
  await page.addInitScript((mockOptions) => {
    const accounts = mockOptions.accounts ?? [];
    const currentAccount = mockOptions.currentAccount ?? null;

    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {
        invoke: async (command: string) => {
          if (command === 'list_codex_accounts') {
            return accounts;
          }
          if (command === 'get_current_codex_account') {
            return currentAccount;
          }
          if (command === 'refresh_all_codex_quotas') {
            return accounts;
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
  }, options);
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

test.describe('Accounts page smoke', () => {
  test('renders empty state and opens import drawer', async ({ page }) => {
    await installTauriMock(page, { accounts: [], currentAccount: null });

    await page.goto('/');

    await expect(page.getByRole('heading', { name: 'No Codex accounts yet' })).toBeVisible();
    await expect(page.getByText('No current Codex account detected.')).toBeVisible();
    await page.getByRole('button', { name: 'Add Account' }).first().click();
    await expect(page.getByRole('dialog', { name: 'Add account' })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test('renders account list with direct actions and quota states', async ({ page }) => {
    await installTauriMock(page, {
      accounts: [oauthAccount, apiKeyAccount],
      currentAccount: oauthAccount,
    });

    await page.goto('/');

    await expect(page.getByRole('heading', { name: 'Codex accounts' })).toBeVisible();
    const oauthRow = page.locator('.account-row').filter({ hasText: 'Work Codex OAuth' });
    const apiKeyRow = page.locator('.account-row').filter({ hasText: 'API Key Only' });

    await expect(oauthRow).toBeVisible();
    await expect(apiKeyRow).toBeVisible();
    await expect(oauthRow.locator('.badge-current')).toHaveText('Current');
    await expect(oauthRow.getByText('Hourly')).toBeVisible();
    await expect(oauthRow.getByText('Weekly')).toBeVisible();
    await expect(oauthRow.getByText('acct_team_codex_lite_smoke_long_identifier_001')).toBeVisible();
    await expect(apiKeyRow.getByLabel('Quota not applicable')).toBeVisible();
    await expect(apiKeyRow.getByRole('button', { name: 'Switch' })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test('supports keyboard tab navigation to primary actions', async ({ page }) => {
    await installTauriMock(page, {
      accounts: [oauthAccount, apiKeyAccount],
      currentAccount: oauthAccount,
    });

    await page.goto('/');
    await page.keyboard.press('Tab');

    const activeLabels = [];
    for (let index = 0; index < 14; index += 1) {
      activeLabels.push(
        await page.evaluate(() => {
          const active = document.activeElement;
          return active instanceof HTMLElement ? active.innerText || active.getAttribute('aria-label') || active.title : '';
        }),
      );
      await page.keyboard.press('Tab');
    }

    expect(activeLabels.some((label) => label.includes('Add Account'))).toBe(true);
    expect(activeLabels.some((label) => label.includes('Refresh quota') || label.includes('Switch'))).toBe(true);
    expect(activeLabels.some((label) => label.includes('Settings'))).toBe(true);
    await expectNoHorizontalOverflow(page);
  });
});
