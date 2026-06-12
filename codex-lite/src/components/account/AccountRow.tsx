import { ChevronRight, RefreshCw, Trash2 } from 'lucide-react';
import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import { QuotaMeter } from './QuotaMeter';
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
    return 'Never';
  }
  return new Date(value * 1000).toLocaleString();
}

function formatReset(value?: number | null): string | null {
  if (!value) {
    return null;
  }
  return new Date(value * 1000).toLocaleString();
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
  const planText = quotaUnsupported ? '-' : account.planType ?? 'Unknown';
  const detailId = `account-detail-${account.id}`;
  const hourly = account.quota?.hourlyRemainingPercent;

  return (
    <article className={`account-row ${account.isCurrent ? 'current' : ''} ${expanded ? 'expanded' : ''}`}>
      <button
        type="button"
        className="account-summary"
        aria-expanded={expanded}
        aria-controls={detailId}
        onClick={() => onToggle(account.id)}
      >
        <span className={`account-status-dot ${account.isCurrent ? 'is-current' : ''}`} aria-hidden="true" />
        <span className="account-avatar" aria-hidden="true">
          {account.displayName.slice(0, 1).toUpperCase()}
        </span>
        <span className="account-title-group">
          <strong>{account.displayName}</strong>
          <span>{account.email ?? account.id}</span>
        </span>
        <span className="account-summary-quota" aria-hidden="true">
          {quotaUnsupported ? (
            <span className="account-summary-quota-na">API Key</span>
          ) : typeof hourly === 'number' ? (
            <>
              <span className="account-summary-quota-track">
                <span
                  className={`account-summary-quota-fill quota-${hourly >= 40 ? 'success' : hourly >= 15 ? 'warning' : 'error'}`}
                  style={{ width: `${Math.max(0, Math.min(100, hourly))}%` }}
                />
              </span>
              <span className="account-summary-quota-value">{Math.max(0, Math.min(100, hourly))}%</span>
            </>
          ) : (
            <span className="account-summary-quota-na">No quota</span>
          )}
        </span>
        <span className="account-badges">
          {account.isCurrent ? <span className="badge badge-current">Current</span> : null}
          {account.quotaError ? <span className="badge badge-error">Error</span> : null}
          <span className="badge">{formatAccountMode(account)}</span>
        </span>
        <ChevronRight className="account-chevron" size={16} aria-hidden="true" />
      </button>

      {expanded ? (
        <div className="account-detail-panel" id={detailId}>
          {quotaUnsupported ? (
            <div className="account-quota-na" aria-label="Quota not applicable">
              <strong>Quota not applicable</strong>
              <span>API Key accounts do not expose ChatGPT web quota.</span>
            </div>
          ) : (
            <div className="account-detail-quotas">
              <QuotaMeter
                error={account.quotaError}
                label="Hourly"
                loading={refreshing}
                resetAt={formatReset(account.quota?.hourlyResetAt)}
                stale={account.quota?.stale ?? false}
                value={account.quota?.hourlyRemainingPercent}
              />
              <QuotaMeter
                error={account.quotaError}
                label="Weekly"
                loading={refreshing}
                resetAt={formatReset(account.quota?.weeklyResetAt)}
                stale={account.quota?.stale ?? false}
                value={account.quota?.weeklyRemainingPercent}
              />
            </div>
          )}

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

          <div className="account-detail-actions">
            <Button
              variant="danger"
              icon={<Trash2 size={16} />}
              loading={deleting}
              onClick={() => onDelete(account.id)}
            >
              Delete
            </Button>
            <div className="account-detail-actions-primary">
              <Button
                variant="ghost"
                icon={<RefreshCw size={16} />}
                disabled={quotaUnsupported}
                loading={refreshing}
                onClick={() => onRefreshQuota(account.id)}
              >
                Refresh quota
              </Button>
              <Button
                variant={account.isCurrent ? 'secondary' : 'primary'}
                disabled={account.isCurrent}
                loading={switching}
                onClick={() => onSwitch(account.id)}
              >
                {account.isCurrent ? 'Current' : 'Switch'}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </article>
  );
}
