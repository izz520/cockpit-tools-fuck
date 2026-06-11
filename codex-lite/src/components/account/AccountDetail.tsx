import { RefreshCw } from 'lucide-react';
import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import { QuotaMeter } from './QuotaMeter';
import './AccountDetail.css';

interface AccountDetailProps {
  account: CodexAccountView | null;
  refreshing: boolean;
  switching: boolean;
  onSwitch: (accountId: string) => void;
  onRefreshQuota: (accountId: string) => void;
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

export function AccountDetail({ account, refreshing, switching, onSwitch, onRefreshQuota }: AccountDetailProps) {
  if (!account) {
    return (
      <div className="account-detail-empty">
        <h2>No account selected</h2>
        <p>Select an account from the list or import your current Codex auth.</p>
      </div>
    );
  }

  const quotaUnsupported = account.authMode === 'api_key';
  const quotaStatusText = account.quota?.updatedAt
    ? `Updated ${formatTimestamp(account.quota.updatedAt)}`
    : 'Not refreshed yet';
  const planText = account.authMode === 'api_key' ? '-' : account.planType ?? 'Unknown';
  const accountIdText = account.authMode === 'api_key' ? '-' : account.accountId ?? 'Unknown';

  return (
    <article className="account-detail">
      <header className="account-detail-header">
        <div>
          <h2>{account.displayName}</h2>
          <p>{account.email ?? account.id}</p>
        </div>
        <div className="detail-actions">
          <Button
            variant="ghost"
            icon={<RefreshCw size={16} />}
            loading={refreshing}
            disabled={quotaUnsupported}
            onClick={() => onRefreshQuota(account.id)}
          >
            Refresh quota
          </Button>
          <Button
            variant="primary"
            loading={switching}
            disabled={account.isCurrent}
            onClick={() => onSwitch(account.id)}
          >
            {account.isCurrent ? 'Already current' : 'Switch'}
          </Button>
        </div>
      </header>

      <section className="detail-section">
        <div className="detail-section-heading">
          <h3>Quota</h3>
          <span className={account.quota?.stale ? 'detail-status detail-status-warning' : 'detail-status'}>
            {quotaUnsupported ? '-' : account.quota?.stale ? 'Stale data' : quotaStatusText}
          </span>
        </div>
        {quotaUnsupported ? (
          <div className="quota-not-applicable">
            <strong>Quota not applicable</strong>
            <span>API Key accounts do not expose ChatGPT web quota.</span>
          </div>
        ) : (
          <div className="detail-quota-grid">
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
          <div className="detail-error" role="alert">
            <strong>{account.quotaError.message}</strong>
            <span>{account.quotaError.action}</span>
          </div>
        ) : null}
      </section>

      <section className="detail-section">
        <h3>Credential summary</h3>
        <dl className="detail-dl">
          <div>
            <dt>Auth mode</dt>
            <dd>{account.authMode === 'api_key' ? 'API Key' : 'OAuth'}</dd>
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
      </section>
    </article>
  );
}
