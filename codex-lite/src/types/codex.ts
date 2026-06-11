import type { AppError } from './system';

export type CodexAuthMode = 'oauth' | 'api_key';

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
  accountId?: string | null;
  userId?: string | null;
  planType?: string | null;
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
