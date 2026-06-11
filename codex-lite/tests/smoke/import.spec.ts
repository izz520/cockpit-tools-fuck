import { expect, test, type Page } from '@playwright/test';

async function installImportMock(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const importedAccount = {
      id: 'imported-api-key',
      displayName: 'Smoke API Key',
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
      createdAt: 1_799_900_000,
      updatedAt: 1_799_900_000,
      lastUsedAt: null,
      isCurrent: false,
      capabilityWarning: 'API key accounts cannot refresh ChatGPT quota.',
    };
    const oauthImportedAccount = {
      ...importedAccount,
      id: 'imported-oauth',
      displayName: 'Smoke OAuth',
      email: 'smoke.oauth@example.test',
      authMode: 'oauth',
      accountId: 'acct_smoke',
      userId: 'user_smoke',
      planType: 'plus',
      apiBaseUrl: null,
      tags: ['oauth'],
      capabilityWarning: null,
    };
    let oauthCallbackSubmitted = false;

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
          if (command === 'add_codex_account_with_api_key') {
            return importedAccount;
          }
          if (command === 'import_codex_from_json') {
            return {
              imported: [importedAccount],
              skipped: [],
              failed: [
                {
                  source:
                    '/Users/smoke/private/codex-lite/imports/really-long-path-that-should-not-overflow-the-import-drawer/auth-with-invalid-token.json',
                  error:
                    'CODEX_AUTH_INVALID_FORMAT: The selected auth JSON has no usable credentials and should stay readable without overflowing.',
                },
                {
                  source: '/Users/smoke/private/codex-lite/imports/another-invalid-auth-file.json',
                  error: 'CODEX_TOKEN_INVALID: The ID token payload could not be decoded.',
                },
              ],
            };
          }
          if (command === 'codex_oauth_login_start') {
            return {
              loginId: 'oauth-smoke-login',
              authUrl: 'https://auth.openai.com/oauth/authorize?client_id=smoke&state=smoke-state',
              redirectUri: 'http://localhost:1455/auth/callback',
              expiresAt: 1_800_000_000,
              listenerStarted: false,
              listenerError: 'Smoke test uses manual callback.',
            };
          }
          if (command === 'is_codex_oauth_port_in_use') {
            return false;
          }
          if (command === 'codex_oauth_submit_callback_url') {
            oauthCallbackSubmitted = true;
            return null;
          }
          if (command === 'codex_oauth_login_status') {
            return { step: oauthCallbackSubmitted ? 'callbackSubmitted' : 'started' };
          }
          if (command === 'codex_oauth_login_completed') {
            return oauthImportedAccount;
          }
          if (command === 'refresh_codex_quota') {
            return {
              ...oauthImportedAccount,
              quota: {
                hourlyRemainingPercent: 90,
                hourlyResetAt: 1_800_000_000,
                weeklyRemainingPercent: 80,
                weeklyResetAt: 1_800_086_400,
                updatedAt: 1_799_990_000,
                stale: false,
              },
            };
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

function addAccountConfirmButton(page: Page) {
  return page.locator('.drawer-footer').getByRole('button', { name: 'Add account', exact: true });
}

test.describe('Import drawer smoke', () => {
  test('imports an API key account from the drawer', async ({ page }) => {
    await installImportMock(page);

    await page.goto('/');
    await page.getByRole('button', { name: 'Add Account' }).first().click();
    await page.getByRole('tab', { name: 'API Key' }).click();
    await page.getByPlaceholder('sk-...').fill('sk-smoke-redacted');
    await page.getByPlaceholder('Optional').first().fill('Smoke API Key');
    await addAccountConfirmButton(page).click();

    await expect(page.getByRole('dialog', { name: 'Add account' })).toBeHidden();
    await expect(page.getByText('Smoke API Key')).toBeVisible();
  });

  test('shows manual OAuth callback flow controls', async ({ page }) => {
    await installImportMock(page);

    await page.goto('/');
    await page.getByRole('button', { name: 'Add Account' }).first().click();
    await page.getByRole('tab', { name: 'OAuth login' }).click();
    await page.getByRole('button', { name: 'Start Login' }).click();

    await expect(page.getByText('oauth-smoke-login')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Open auth URL' })).toHaveAttribute('href', /auth\.openai\.com/);

    await page
      .getByPlaceholder('http://localhost:1455/auth/callback?code=...&state=...')
      .fill('http://localhost:1455/auth/callback?code=smoke-code&state=smoke-state');
    await page.getByRole('button', { name: 'Submit Callback' }).click();

    await expect(page.getByRole('dialog', { name: 'Add account' })).toBeHidden();
    await expect(page.getByText('Smoke OAuth')).toBeVisible();
  });

  test('shows source-specific forms and partial failure results', async ({ page }) => {
    await installImportMock(page);

    await page.goto('/');
    await page.getByRole('button', { name: 'Add Account' }).first().click();

    await page.getByRole('tab', { name: 'Current local auth' }).click();
    await expect(page.getByText('Ready to add your current local Codex auth.')).toBeVisible();

    await page.getByRole('tab', { name: 'JSON text' }).click();
    await expect(page.getByPlaceholder('{"auth_mode":"oauth","tokens":{...}}')).toBeVisible();
    await page.getByPlaceholder('{"auth_mode":"oauth","tokens":{...}}').fill('{"authMode":"oauth"}');
    await addAccountConfirmButton(page).click();

    await expect(page.getByRole('list', { name: 'Added accounts' })).toContainText('Smoke API Key');
    await expect(page.getByText('Added 1, failed 2')).toBeVisible();
    await expect(page.getByRole('list', { name: 'Failed imports' })).toContainText('auth-with-invalid-token.json');
    await expect(page.getByRole('list', { name: 'Failed imports' })).toContainText('another-invalid-auth-file.json');
    await expect(page.getByText(/CODEX_AUTH_INVALID_FORMAT/)).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.getByRole('tab', { name: 'Token' }).click();
    await expect(page.getByPlaceholder('Paste id_token')).toBeVisible();

    await page.getByRole('tab', { name: 'JSON file' }).click();
    await expect(page.getByRole('button', { name: 'Choose JSON' })).toBeVisible();
  });
});
