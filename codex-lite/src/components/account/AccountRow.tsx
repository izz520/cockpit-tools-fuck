import { CalendarDays, Clock3, Copy, Eye, KeyRound, Link, Play, RefreshCw, ShieldCheck, Trash2 } from 'lucide-react';
import { isOAuthAuthMode, type CodexAccountView } from '../../types/codex';
import type { AppError } from '../../types/system';
import { Button } from '../ui/Button';
import { IconButton } from '../ui/IconButton';
import './AccountRow.css';

interface AccountRowProps {
  account: CodexAccountView;
  refreshing: boolean;
  switching: boolean;
  deleting: boolean;
  onRefreshQuota: (accountId: string) => void;
  onSwitch: (accountId: string) => void;
  onDelete: (accountId: string) => void;
  onReauthenticate: (accountId: string) => void;
}

function formatTimestamp(value?: number | null): string {
  if (!value) {
    return '从未使用';
  }
  return new Date(value * 1000).toLocaleDateString();
}

function formatReset(value?: number | null): string | null {
  if (!value) {
    return null;
  }
  return new Date(value * 1000).toLocaleString();
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

function getApiBaseUrl(account: CodexAccountView): string {
  return account.apiBaseUrl ?? 'https://api.openai.com/v1';
}

function getOAuthBindingText(account: CodexAccountView): string {
  if (!account.email) {
    return '绑定';
  }
  return account.email;
}

export function AccountRow({
  account,
  refreshing,
  switching,
  deleting,
  onRefreshQuota,
  onSwitch,
  onDelete,
  onReauthenticate,
}: AccountRowProps) {
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
                  <button type="button" aria-label="Copy API key" disabled>
                    <Copy size={13} />
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
                    aria-label="Copy API base URL"
                    onClick={() => void navigator.clipboard?.writeText(getApiBaseUrl(account))}
                  >
                    <Copy size={13} />
                  </button>
                </span>
                <code>{getApiBaseUrl(account)}</code>
              </div>
            </div>
            <span className="api-oauth-binding">
              <span>
                <ShieldCheck size={14} />
                OAuth {account.email ? '已绑定' : '未绑定'}
              </span>
              <span>{getOAuthBindingText(account)}</span>
            </span>
          </>
        ) : hasQuotaError && quotaErrorSummary ? (
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

        {!quotaUnsupported ? (
          <span className="account-validity">
            <span>
              <CalendarDays size={15} />
              有效期 30 天
            </span>
            <span>{formatReset(account.quota?.weeklyResetAt) ?? formatTimestamp(account.lastUsedAt)}</span>
          </span>
        ) : null}
      </div>

      <div className="account-card-actions">
        <IconButton
          label={account.isCurrent ? 'Current account' : 'Switch account'}
          icon={switching ? <RefreshCw className="spin-icon" size={18} /> : <Play size={18} />}
          active={account.isCurrent}
          disabled={account.isCurrent || switching}
          onClick={() => onSwitch(account.id)}
        />
        {!quotaUnsupported ? (
          <IconButton
            label="Refresh quota"
            icon={refreshing ? <RefreshCw className="spin-icon" size={18} /> : <RefreshCw size={18} />}
            disabled={refreshing}
            onClick={() => onRefreshQuota(account.id)}
          />
        ) : null}
        <IconButton
          label="Delete account"
          icon={deleting ? <RefreshCw className="spin-icon" size={18} /> : <Trash2 size={18} />}
          disabled={deleting}
          onClick={() => onDelete(account.id)}
        />
      </div>
    </article>
  );
}
