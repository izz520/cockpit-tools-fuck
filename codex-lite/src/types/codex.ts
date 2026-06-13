import type { AppError } from './system';

export type CodexAuthMode = 'oauth' | 'o_auth' | 'api_key';

export function isOAuthAuthMode(authMode: CodexAuthMode): boolean {
  return authMode === 'oauth' || authMode === 'o_auth';
}

export interface CodexQuotaView {
  hourlyRemainingPercent?: number | null;
  hourlyResetAt?: number | null;
  weeklyRemainingPercent?: number | null;
  weeklyResetAt?: number | null;
  updatedAt?: number | null;
  stale: boolean;
}

export interface CodexAccountView {
  id: string;
  displayName: string;
  email?: string | null;
  authMode: CodexAuthMode;
  boundOauthAccountId?: string | null;
  accountId?: string | null;
  userId?: string | null;
  planType?: string | null;
  apiKey?: string | null;
  apiBaseUrl?: string | null;
  quota?: CodexQuotaView | null;
  quotaError?: AppError | null;
  tags: string[];
  note?: string | null;
  createdAt: number;
  updatedAt: number;
  lastUsedAt?: number | null;
  isCurrent: boolean;
  capabilityWarning?: string | null;
}

export interface SwitchResult {
  account: CodexAccountView;
  backupPath?: string | null;
  restored: boolean;
}
