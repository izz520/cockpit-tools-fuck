import type { CodexAccountView, SwitchResult } from '../types/codex';
import { invokeCommand } from './tauriInvoke';

export function listCodexAccounts(): Promise<CodexAccountView[]> {
  return invokeCommand('list_codex_accounts');
}

export function getCurrentCodexAccount(): Promise<CodexAccountView | null> {
  return invokeCommand('get_current_codex_account');
}

export function switchCodexAccount(accountId: string): Promise<SwitchResult> {
  return invokeCommand('switch_codex_account', { accountId });
}

export function refreshCodexQuota(accountId: string): Promise<CodexAccountView> {
  return invokeCommand('refresh_codex_quota', { accountId });
}

export function refreshAllCodexQuotas(): Promise<CodexAccountView[]> {
  return invokeCommand('refresh_all_codex_quotas');
}

export function deleteCodexAccount(accountId: string): Promise<void> {
  return invokeCommand('delete_codex_account', { accountId });
}

export function updateCodexApiKeyAccount(
  accountId: string,
  apiKey: string,
  apiBaseUrl: string | null,
  displayName: string | null,
): Promise<CodexAccountView> {
  return invokeCommand('update_codex_api_key_account', { accountId, apiKey, apiBaseUrl, displayName });
}

export function updateCodexApiKeyBoundOAuthAccount(
  accountId: string,
  boundOauthAccountId: string | null,
): Promise<CodexAccountView> {
  return invokeCommand('update_codex_api_key_bound_oauth_account', { accountId, boundOauthAccountId });
}
