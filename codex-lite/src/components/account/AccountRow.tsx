import { CalendarDays, Check, Clock3, Copy, Eye, KeyRound, Link, Link2, Pencil, Play, RefreshCw, ShieldCheck, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { isOAuthAuthMode, type CodexAccountView } from '../../types/codex';
import type { AppError } from '../../types/system';
import { Button } from '../ui/Button';
import { IconButton } from '../ui/IconButton';
import './AccountRow.css';

interface AccountRowProps {
  account: CodexAccountView;
  boundOAuthAccount: CodexAccountView | null;
  refreshing: boolean;
  switching: boolean;
  deleting: boolean;
  onRefreshQuota: (accountId: string) => void;
  onSwitch: (accountId: string) => void;
  onDelete: (accountId: string) => void;
  onEditApiAccount: (account: CodexAccountView) => void;
  onBindOAuthAccount: (account: CodexAccountView) => void;
  onReauthenticate: (accountId: string) => void;
}

function parseSubscriptionTimestamp(value?: string | null): number | null {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const numeric = Number(trimmed);
  if (Number.isFinite(numeric)) {
    return numeric > 10_000_000_000 ? numeric : numeric * 1000;
  }
  const parsed = Date.parse(trimmed);
  return Number.isNaN(parsed) ? null : parsed;
}

function formatExpiry(value?: string | null): string | null {
  const timestamp = parseSubscriptionTimestamp(value);
  if (!timestamp) {
    return null;
  }
  return new Date(timestamp).toLocaleString();
}

function formatRemainingDays(value?: string | null): string {
  const timestamp = parseSubscriptionTimestamp(value);
  if (!timestamp) {
    return '-';
  }
  const msRemaining = timestamp - Date.now();
  return String(Math.max(0, Math.ceil(msRemaining / 86_400_000)));
}

function formatQuotaLabel(value?: number | null): string {
  if (typeof value !== 'number') {
    return '-';
  }
  return `${Math.max(0, Math.min(100, value))}%`;
}

function getPlanTone(planType?: string | null): string {
  const plan = planType?.toLowerCase() ?? '';
  if (plan.includes('pro')) {
    return 'pro';
  }
  if (plan.includes('team')) {
    return 'team';
  }
  if (plan.includes('plus')) {
    return 'plus';
  }
  return 'default';
}

function canReauthenticateAccount(account: CodexAccountView): boolean {
  return isOAuthAuthMode(account.authMode) && account.quotaError !== null && account.quotaError !== undefined;
}

function getQuotaErrorSummary(error: AppError): { title: string; action: string } {
  const text = `${error.code} ${error.message}`.toLowerCase();
  if (
    error.code === 'CODEX_QUOTA_UNAUTHORIZED' ||
    text.includes('token_invalidated') ||
    text.includes('token_revoked') ||
    text.includes('authentication token has been invalidated')
  ) {
    return {
      title: '授权已失效',
      action: '请重新授权后再刷新额度。',
    };
  }

  if (text.includes('http 401') || text.includes('http 403')) {
    return {
      title: '额度接口认证失败',
      action: '请重新授权或检查账号状态。',
    };
  }

  return {
    title: error.message.split('.')[0] || '额度刷新失败',
    action: error.action,
  };
}

function getMaskedApiKey(): string {
  return 'sk-********************';
}

function getCopyableApiKey(account: CodexAccountView): string {
  return account.apiKey ?? '';
}

function getApiBaseUrl(account: CodexAccountView): string {
  return account.apiBaseUrl ?? 'https://api.openai.com/v1';
}

function getOAuthBindingText(boundOAuthAccount: CodexAccountView | null): string {
  if (!boundOAuthAccount) {
    return '未绑定';
  }
  return boundOAuthAccount.email ?? boundOAuthAccount.displayName;
}

export function AccountRow({
  account,
  boundOAuthAccount,
  refreshing,
  switching,
  deleting,
  onRefreshQuota,
  onSwitch,
  onDelete,
  onEditApiAccount,
  onBindOAuthAccount,
  onReauthenticate,
}: AccountRowProps) {
  const [copiedField, setCopiedField] = useState<'apiKey' | 'apiBaseUrl' | null>(null);
  const quotaUnsupported = account.authMode === 'api_key';
  const planText = quotaUnsupported ? 'API' : account.planType ?? 'Free';
  const hourly = account.quota?.hourlyRemainingPercent;
  const weekly = account.quota?.weeklyRemainingPercent;
  const normalizedHourly = typeof hourly === 'number' ? Math.max(0, Math.min(100, hourly)) : 0;
  const normalizedWeekly = typeof weekly === 'number' ? Math.max(0, Math.min(100, weekly)) : 0;
  const planTone = quotaUnsupported ? 'api' : getPlanTone(account.planType);
  const canReauthenticate = canReauthenticateAccount(account);
  const hasQuotaError = account.quotaError !== null && account.quotaError !== undefined;
  const quotaErrorSummary = account.quotaError ? getQuotaErrorSummary(account.quotaError) : null;
  const expiresAt = account.subscriptionActiveUntil ?? null;
  const validityDays = formatRemainingDays(expiresAt);
  const expiresAtText = formatExpiry(expiresAt) ?? '未获取到期时间';

  async function copyField(field: 'apiKey' | 'apiBaseUrl', value: string) {
    if (!value) {
      return;
    }
    await navigator.clipboard.writeText(value);
    setCopiedField(field);
    window.setTimeout(() => setCopiedField(null), 1300);
  }

  return (
    <article className={`account-card account-row ${quotaUnsupported ? 'api-account-card' : ''} ${account.isCurrent ? 'current' : ''}`}>
      <div className="account-card-main account-summary">
        <span className="account-card-header">
          <strong>{account.email ?? account.displayName}</strong>
          <span className={`account-plan-badge account-plan-${planTone}`}>{planText}</span>
        </span>
        {!quotaUnsupported ? <span className="account-card-name">{account.displayName}</span> : null}

        {quotaUnsupported ? (
          <>
            <div className="api-account-fields">
              <div className="api-account-field">
                <span className="api-account-field-label">
                  <KeyRound size={14} />
                  API Key
                </span>
                <span className="api-account-field-actions">
                  <button type="button" aria-label="Show API key" disabled>
                    <Eye size={13} />
                  </button>
                  <button
                    type="button"
                    aria-label={copiedField === 'apiKey' ? 'API key copied' : 'Copy API key'}
                    className={copiedField === 'apiKey' ? 'copied' : ''}
                    title={copiedField === 'apiKey' ? '复制成功' : '复制 API Key'}
                    disabled={!account.apiKey}
                    onClick={() => void copyField('apiKey', getCopyableApiKey(account))}
                  >
                    {copiedField === 'apiKey' ? <Check size={13} /> : <Copy size={13} />}
                  </button>
                </span>
                <code>{getMaskedApiKey()}</code>
              </div>
              <div className="api-account-field">
                <span className="api-account-field-label">
                  <Link size={14} />
                  基础地址
                </span>
                <span className="api-account-field-actions">
                  <button
                    type="button"
                    aria-label={copiedField === 'apiBaseUrl' ? 'API base URL copied' : 'Copy API base URL'}
                    className={copiedField === 'apiBaseUrl' ? 'copied' : ''}
                    title={copiedField === 'apiBaseUrl' ? '复制成功' : '复制基础地址'}
                    onClick={() => void copyField('apiBaseUrl', getApiBaseUrl(account))}
                  >
                    {copiedField === 'apiBaseUrl' ? <Check size={13} /> : <Copy size={13} />}
                  </button>
                </span>
                <code>{getApiBaseUrl(account)}</code>
              </div>
            </div>
            <span className="api-oauth-binding">
              <span>
                <ShieldCheck size={14} />
                OAuth {boundOAuthAccount ? '已绑定' : '未绑定'}
              </span>
              <button type="button" onClick={() => onBindOAuthAccount(account)}>
                <Link2 size={13} />
                <span>{getOAuthBindingText(boundOAuthAccount)}</span>
              </button>
            </span>
          </>
        ) : (
          <div className="account-health-slot">
            {hasQuotaError && quotaErrorSummary ? (
              <div className="account-detail-error" role="alert">
                <span className="account-detail-error-body">
                  <strong>{quotaErrorSummary.title}</strong>
                  <span>{quotaErrorSummary.action}</span>
                </span>
                {canReauthenticate ? (
                  <span className="account-detail-error-actions">
                    <Button variant="secondary" icon={<KeyRound size={14} />} onClick={() => onReauthenticate(account.id)}>
                      重新授权
                    </Button>
                  </span>
                ) : null}
              </div>
            ) : (
              <span className="quota-lines" aria-hidden="true">
                <span className="quota-line">
                  <span>
                    <Clock3 size={16} />
                    5h
                  </span>
                  <strong>{quotaUnsupported ? 'API' : formatQuotaLabel(hourly)}</strong>
                </span>
                <span className="quota-track">
                  <span style={{ width: `${quotaUnsupported ? 100 : normalizedHourly}%` }} />
                </span>
                <span className="quota-line">
                  <span>
                    <CalendarDays size={16} />
                    Weekly
                  </span>
                  <strong>{quotaUnsupported ? '可用' : formatQuotaLabel(weekly)}</strong>
                </span>
                <span className="quota-track">
                  <span style={{ width: `${quotaUnsupported ? 100 : normalizedWeekly}%` }} />
                </span>
              </span>
            )}
          </div>
        )}

        {!quotaUnsupported ? (
          <span className="account-validity">
            <span>
              <CalendarDays size={15} />
              有效期 {validityDays} 天
            </span>
            <span>{expiresAtText}</span>
          </span>
        ) : null}
      </div>

      <div className="account-card-actions">
        <IconButton
          label={account.isCurrent ? 'Current account' : 'Switch account'}
          icon={switching ? <RefreshCw className="spin-icon" size={16} /> : <Play size={16} />}
          active={account.isCurrent}
          disabled={account.isCurrent || switching}
          onClick={() => onSwitch(account.id)}
        />
        {!quotaUnsupported ? (
          <IconButton
            label="Refresh quota"
            icon={refreshing ? <RefreshCw className="spin-icon" size={16} /> : <RefreshCw size={16} />}
            disabled={refreshing}
            onClick={() => onRefreshQuota(account.id)}
          />
        ) : null}
        {quotaUnsupported ? (
          <IconButton label="Edit API account" icon={<Pencil size={16} />} onClick={() => onEditApiAccount(account)} />
        ) : null}
        <IconButton
          label="Delete account"
          icon={deleting ? <RefreshCw className="spin-icon" size={16} /> : <Trash2 size={16} />}
          disabled={deleting}
          onClick={() => onDelete(account.id)}
        />
      </div>
    </article>
  );
}
