import { useEffect, useMemo, useState } from 'react';
import { RefreshCw, UserPlus } from 'lucide-react';
import { AccountRow } from '../components/account/AccountRow';
import { ConfirmSwitchModal } from '../components/account/ConfirmSwitchModal';
import { ImportDrawer } from '../components/import/ImportDrawer';
import { Button } from '../components/ui/Button';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { useCodexAccountsStore } from '../stores/useCodexAccountsStore';
import { useImportFlowStore } from '../stores/useImportFlowStore';
import './AccountsPage.css';

export function AccountsPage() {
  const {
    accounts,
    currentAccountId,
    loading,
    error,
    refreshingAll,
    refreshingAccountIds,
    switchingAccountId,
    loadAccounts,
    refreshAccountQuota,
    refreshAllQuotas,
    switchToAccount,
  } = useCodexAccountsStore();
  const openImportDrawer = useImportFlowStore((state) => state.openDrawer);
  const [pendingSwitchAccountId, setPendingSwitchAccountId] = useState<string | null>(null);

  useEffect(() => {
    void loadAccounts();
  }, [loadAccounts]);

  const oauthAccountCount = useMemo(
    () => accounts.filter((account) => account.authMode === 'oauth').length,
    [accounts],
  );
  const pendingSwitchAccount = useMemo(
    () => accounts.find((account) => account.id === pendingSwitchAccountId) ?? null,
    [accounts, pendingSwitchAccountId],
  );

  async function confirmSwitch(accountId: string) {
    await switchToAccount(accountId);
    setPendingSwitchAccountId(null);
  }

  return (
    <>
      <div className="content">
        <section className="panel accounts-panel">
          {error ? <ErrorBanner error={error} /> : null}
          <div className="accounts-panel-header">
            <div>
              <h2 className="section-title">Accounts</h2>
              <p className="muted">
                Current:{' '}
                {currentAccountId
                  ? accounts.find((account) => account.id === currentAccountId)?.displayName ?? currentAccountId
                  : 'No current Codex account detected.'}
              </p>
            </div>
            <div className="accounts-panel-actions">
              <Button
                variant="secondary"
                icon={<RefreshCw size={16} />}
                loading={refreshingAll}
                disabled={loading || oauthAccountCount === 0}
                onClick={() => void refreshAllQuotas()}
              >
                Refresh all
              </Button>
              <Button variant="primary" icon={<UserPlus size={16} />} onClick={openImportDrawer}>
                Add Account
              </Button>
            </div>
          </div>
          <div className="account-list-tools">
            <input aria-label="Search accounts" placeholder="Search accounts" />
          </div>
          <div className="account-list">
            {loading ? <p className="account-list-message">Loading accounts...</p> : null}
            {!loading && accounts.length === 0 ? (
              <div className="empty-state">
                <h2>No Codex accounts yet</h2>
                <p>Add your current local Codex auth or another account.</p>
                <Button variant="primary" icon={<UserPlus size={16} />} onClick={openImportDrawer}>
                  Add Account
                </Button>
              </div>
            ) : null}
            {accounts.map((account) => (
              <AccountRow
                account={account}
                key={account.id}
                refreshing={refreshingAll || refreshingAccountIds.includes(account.id)}
                switching={switchingAccountId === account.id}
                onRefreshQuota={(accountId) => void refreshAccountQuota(accountId)}
                onSwitch={setPendingSwitchAccountId}
              />
            ))}
          </div>
        </section>
      </div>
      <ImportDrawer />
      <ConfirmSwitchModal
        account={pendingSwitchAccount}
        switching={switchingAccountId === pendingSwitchAccountId}
        onCancel={() => setPendingSwitchAccountId(null)}
        onConfirm={(accountId) => void confirmSwitch(accountId)}
      />
    </>
  );
}
