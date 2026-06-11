import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import { QuotaMeter } from './QuotaMeter';
import './AccountRow.css';

interface AccountRowProps {
  account: CodexAccountView;
  refreshing: boolean;
  switching: boolean;
  onRefreshQuota: (accountId: string) => void;
  onSwitch: (accountId: string) => void;
}

function formatAccountMode(account: CodexAccountView): string {
  return account.authMode === 'api_key' ? 'API Key' : 'OAuth';
}

export function AccountRow({ account, refreshing, switching, onRefreshQuota, onSwitch }: AccountRowProps) {
  const quotaUnsupported = account.authMode === 'api_key';
  const accountIdText = account.authMode === 'api_key' ? '-' : account.accountId ?? 'Unknown';
  const planText = account.authMode === 'api_key' ? '-' : account.planType ?? 'Unknown';

  return (
    <article className={`account-row ${account.isCurrent ? 'current' : ''}`}>
      <div className="account-avatar">{account.displayName.slice(0, 1).toUpperCase()}</div>
      <div className="account-row-main">
        <div className="account-row-header">
          <div className="account-title-group">
            <strong>{account.displayName}</strong>
            <span>{account.email ?? account.id}</span>
          </div>
          <div className="account-badges">
            {account.isCurrent ? <span className="badge badge-current">Current</span> : null}
            {account.quotaError ? <span className="badge badge-error">Error</span> : null}
            <span className="badge">{formatAccountMode(account)}</span>
          </div>
        </div>

        <div className="account-row-body">
          <dl className="account-meta-grid">
            <div>
              <dt>Plan</dt>
              <dd>{planText}</dd>
            </div>
            <div>
              <dt>Account ID</dt>
              <dd className="mono">{accountIdText}</dd>
            </div>
          </dl>

          {quotaUnsupported ? (
            <div className="account-quota-na" aria-label="Quota not applicable">
              <span>Quota</span>
              <strong>-</strong>
            </div>
          ) : (
            <div className="account-quotas">
              <QuotaMeter label="Hourly" loading={refreshing} value={account.quota?.hourlyRemainingPercent} />
              <QuotaMeter label="Weekly" loading={refreshing} value={account.quota?.weeklyRemainingPercent} />
            </div>
          )}

          <div className="account-row-actions">
            <Button
              variant="ghost"
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
    </article>
  );
}
