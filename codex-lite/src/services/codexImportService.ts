import type { CodexAccountView } from '../types/codex';
import type { BatchImportSession, ImportResult, OAuthStartResult, OAuthStatusResult } from '../types/import';
import { open } from '@tauri-apps/plugin-dialog';
import { invokeCommand } from './tauriInvoke';

export async function chooseCodexAuthFiles(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [
      {
        name: 'Codex auth JSON',
        extensions: ['json'],
      },
    ],
  });

  if (selected === null) {
    return [];
  }

  return Array.isArray(selected) ? selected : [selected];
}

export function importCodexFromLocal(): Promise<CodexAccountView> {
  return invokeCommand('import_codex_from_local');
}

export function importCodexFromJson(jsonContent: string): Promise<ImportResult> {
  return invokeCommand('import_codex_from_json', { jsonContent });
}

export function importCodexFromFiles(filePaths: string[]): Promise<ImportResult> {
  return invokeCommand('import_codex_from_files', { filePaths });
}

export function addCodexAccountWithToken(
  idToken: string,
  accessToken: string,
  refreshToken?: string,
): Promise<CodexAccountView> {
  return invokeCommand('add_codex_account_with_token', {
    idToken,
    accessToken,
    refreshToken: refreshToken ?? null,
  });
}

export function addCodexAccountWithApiKey(
  apiKey: string,
  apiBaseUrl?: string,
  displayName?: string,
): Promise<CodexAccountView> {
  return invokeCommand('add_codex_account_with_api_key', {
    apiKey,
    apiBaseUrl: apiBaseUrl ?? null,
    displayName: displayName ?? null,
  });
}

export function startCodexOAuthLogin(): Promise<OAuthStartResult> {
  return invokeCommand('codex_oauth_login_start');
}

export function submitCodexOAuthCallbackUrl(loginId: string, callbackUrl: string): Promise<void> {
  return invokeCommand('codex_oauth_submit_callback_url', { loginId, callbackUrl });
}

export function getCodexOAuthLoginStatus(loginId: string): Promise<OAuthStatusResult> {
  return invokeCommand('codex_oauth_login_status', { loginId });
}

export function completeCodexOAuthLogin(loginId: string): Promise<CodexAccountView> {
  return invokeCommand('codex_oauth_login_completed', { loginId });
}

export function cancelCodexOAuthLogin(loginId?: string): Promise<void> {
  return invokeCommand('codex_oauth_login_cancel', { loginId: loginId ?? null });
}

export function isCodexOAuthPortInUse(): Promise<boolean> {
  return invokeCommand('is_codex_oauth_port_in_use');
}

export function startCodexBatchImportFromFiles(
  filePaths: string[],
  checkQuota: boolean,
): Promise<BatchImportSession> {
  return invokeCommand('start_codex_batch_import_from_files', { filePaths, checkQuota });
}

export function confirmCodexBatchImport(sessionId: string, itemIds: string[]): Promise<ImportResult> {
  return invokeCommand('confirm_codex_batch_import', { sessionId, itemIds });
}
