import { CalendarDays, Clock3, KeyRound, Play, RefreshCw, Trash2 } from 'lucide-react';
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
    <article className={`account-card account-row ${account.isCurrent ? 'current' : ''}`}>
      <div className="account-card-main account-summary">
        <span className="account-card-header">
          <strong>{account.email ?? account.displayName}</strong>
          <span className={`account-plan-badge account-plan-${planTone}`}>{planText}</span>
        </span>
        <span className="account-card-name">{account.displayName}</span>

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

        <span className="account-validity">
          <span>
            <CalendarDays size={15} />
            有效期 30 天
          </span>
          <span>{formatReset(account.quota?.weeklyResetAt) ?? formatTimestamp(account.lastUsedAt)}</span>
        </span>
      </div>

      <div className="account-card-actions">
        <IconButton
          label={account.isCurrent ? 'Current account' : 'Switch account'}
          icon={switching ? <RefreshCw className="spin-icon" size={18} /> : <Play size={18} />}
          active={account.isCurrent}
          disabled={account.isCurrent || switching}
          onClick={() => onSwitch(account.id)}
        />
        <IconButton
          label="Refresh quota"
          icon={refreshing ? <RefreshCw className="spin-icon" size={18} /> : <RefreshCw size={18} />}
          disabled={quotaUnsupported || refreshing}
          onClick={() => onRefreshQuota(account.id)}
        />
        <IconButton
          label="Delete account"
          icon={deleting ? <RefreshCw className="spin-icon" size={18} /> : <Trash2 size={18} />}
          disabled={deleting}
          onClick={() => onDelete(account.id)}
        />
        {quotaUnsupported ? <KeyRound className="account-card-mode" size={16} aria-label="API Key" /> : null}
      </div>
    </article>
  );
}
