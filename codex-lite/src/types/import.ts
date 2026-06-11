import type { CodexAccountView } from './codex';

export interface ImportFailure {
  source: string;
  error: string;
}

export interface ImportResult {
  imported: CodexAccountView[];
  skipped: CodexAccountView[];
  failed: ImportFailure[];
}

export interface OAuthStartResult {
  loginId: string;
  authUrl: string;
  redirectUri: string;
  expiresAt: number;
  listenerStarted: boolean;
  listenerError?: string | null;
}

export interface OAuthStatusResult {
  step: 'started' | 'callbackSubmitted' | 'expired' | 'missing';
}

export interface BatchImportItem {
  id: string;
  source: string;
  selected: boolean;
  selectable: boolean;
  reason?: string | null;
  accountId?: string | null;
  userId?: string | null;
  displayName?: string | null;
  email?: string | null;
  authMode?: CodexAccountView['authMode'] | null;
  planType?: string | null;
  apiBaseUrl?: string | null;
  quota?: CodexAccountView['quota'] | null;
  quotaWarning?: string | null;
  status: 'importable' | 'existing' | 'failed';
}

export interface BatchImportSession {
  sessionId: string;
  createdAt: number;
  expiresAt: number;
  checkQuota: boolean;
  items: BatchImportItem[];
}
