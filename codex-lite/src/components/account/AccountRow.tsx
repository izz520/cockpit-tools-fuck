import { CalendarDays, Clock3, KeyRound, Play, RefreshCw, Trash2 } from 'lucide-react';
import type { CodexAccountView } from '../../types/codex';
import { IconButton } from '../ui/IconButton';
import './AccountRow.css';

interface AccountRowProps {
  account: CodexAccountView;
  expanded: boolean;
  refreshing: boolean;
  switching: boolean;
  deleting: boolean;
  onToggle: (accountId: string) => void;
  onRefreshQuota: (accountId: string) => void;
  onSwitch: (accountId: string) => void;
  onDelete: (accountId: string) => void;
}

function formatAccountMode(account: CodexAccountView): string {
  return account.authMode === 'api_key' ? 'API Key' : 'OAuth';
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

export function AccountRow({
  account,
  expanded,
  refreshing,
  switching,
  deleting,
  onToggle,
  onRefreshQuota,
  onSwitch,
  onDelete,
}: AccountRowProps) {
  const quotaUnsupported = account.authMode === 'api_key';
  const accountIdText = quotaUnsupported ? '-' : account.accountId ?? 'Unknown';
  const planText = quotaUnsupported ? 'API' : account.planType ?? 'Free';
  const detailId = `account-detail-${account.id}`;
  const hourly = account.quota?.hourlyRemainingPercent;
  const weekly = account.quota?.weeklyRemainingPercent;
  const normalizedHourly = typeof hourly === 'number' ? Math.max(0, Math.min(100, hourly)) : 0;
  const normalizedWeekly = typeof weekly === 'number' ? Math.max(0, Math.min(100, weekly)) : 0;
  const planTone = quotaUnsupported ? 'api' : getPlanTone(account.planType);

  return (
    <article className={`account-card account-row ${account.isCurrent ? 'current' : ''} ${expanded ? 'expanded' : ''}`}>
      <button
        type="button"
        className="account-card-main account-summary"
        aria-expanded={expanded}
        aria-controls={detailId}
        onClick={() => onToggle(account.id)}
      >
        <span className="account-card-header">
          <strong>{account.email ?? account.displayName}</strong>
          <span className={`account-plan-badge account-plan-${planTone}`}>{planText}</span>
        </span>
        <span className="account-card-name">{account.displayName}</span>

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

        <span className="account-validity">
          <span>
            <CalendarDays size={15} />
            有效期 30 天
          </span>
          <span>{formatReset(account.quota?.weeklyResetAt) ?? formatTimestamp(account.lastUsedAt)}</span>
        </span>
      </button>

      {expanded ? (
        <div className="account-detail-panel" id={detailId}>
          {account.quotaError ? (
            <div className="account-detail-error" role="alert">
              <strong>{account.quotaError.message}</strong>
              <span>{account.quotaError.action}</span>
            </div>
          ) : null}

          <dl className="account-detail-dl">
            <div>
              <dt>Auth mode</dt>
              <dd>{formatAccountMode(account)}</dd>
            </div>
            <div>
              <dt>Plan</dt>
              <dd>{planText}</dd>
            </div>
            <div>
              <dt>Account ID</dt>
              <dd className="mono">{accountIdText}</dd>
            </div>
            <div>
              <dt>Last used</dt>
              <dd>{formatTimestamp(account.lastUsedAt)}</dd>
            </div>
          </dl>
        </div>
      ) : null}

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
