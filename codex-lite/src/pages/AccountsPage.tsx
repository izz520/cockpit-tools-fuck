import { useEffect, useMemo, useState } from 'react';
import { RefreshCw, UserPlus } from 'lucide-react';
import { AccountRow } from '../components/account/AccountRow';
import { ConfirmSwitchModal } from '../components/account/ConfirmSwitchModal';
import { ConfirmDeleteModal } from '../components/account/ConfirmDeleteModal';
import { ImportDrawer } from '../components/import/ImportDrawer';
import { Button } from '../components/ui/Button';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { useCodexAccountsStore } from '../stores/useCodexAccountsStore';
import { useImportFlowStore } from '../stores/useImportFlowStore';
import type { CodexAccountView } from '../types/codex';
import './AccountsPage.css';

function matchesQuery(account: CodexAccountView, query: string): boolean {
  const haystack = [account.displayName, account.email, account.accountId, account.id]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  return haystack.includes(query);
}

export function AccountsPage() {
  const {
    accounts,
    currentAccountId,
    selectedAccountId,
    loading,
    error,
    refreshingAll,
    refreshingAccountIds,
    switchingAccountId,
    deletingAccountId,
    loadAccounts,
    selectAccount,
    refreshAccountQuota,
    refreshAllQuotas,
    switchToAccount,
    deleteAccount,
  } = useCodexAccountsStore();
  const openImportDrawer = useImportFlowStore((state) => state.openDrawer);
  const [pendingSwitchAccountId, setPendingSwitchAccountId] = useState<string | null>(null);
  const [pendingDeleteAccountId, setPendingDeleteAccountId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    void loadAccounts();
  }, [loadAccounts]);

  const oauthAccountCount = useMemo(
    () => accounts.filter((account) => account.authMode === 'oauth').length,
    [accounts],
  );
  const filteredAccounts = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (query.length === 0) {
      return accounts;
    }
    return accounts.filter((account) => matchesQuery(account, query));
  }, [accounts, searchQuery]);
  const pendingSwitchAccount = useMemo(
    () => accounts.find((account) => account.id === pendingSwitchAccountId) ?? null,
    [accounts, pendingSwitchAccountId],
  );
  const pendingDeleteAccount = useMemo(
    () => accounts.find((account) => account.id === pendingDeleteAccountId) ?? null,
    [accounts, pendingDeleteAccountId],
  );

  function toggleExpanded(accountId: string) {
    selectAccount(selectedAccountId === accountId ? '' : accountId);
  }

  async function confirmSwitch(accountId: string) {
    await switchToAccount(accountId);
    setPendingSwitchAccountId(null);
  }

  async function confirmDelete(accountId: string) {
    await deleteAccount(accountId);
    setPendingDeleteAccountId(null);
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
            <input
              aria-label="Search accounts"
              placeholder="Search accounts"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
            />
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
            {!loading && accounts.length > 0 && filteredAccounts.length === 0 ? (
              <p className="account-list-message">No accounts match "{searchQuery.trim()}".</p>
            ) : null}
            {filteredAccounts.map((account) => (
              <AccountRow
                account={account}
                key={account.id}
                expanded={selectedAccountId === account.id}
                refreshing={refreshingAll || refreshingAccountIds.includes(account.id)}
                switching={switchingAccountId === account.id}
                deleting={deletingAccountId === account.id}
                onToggle={toggleExpanded}
                onRefreshQuota={(accountId) => void refreshAccountQuota(accountId)}
                onSwitch={setPendingSwitchAccountId}
                onDelete={setPendingDeleteAccountId}
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
      <ConfirmDeleteModal
        account={pendingDeleteAccount}
        deleting={deletingAccountId === pendingDeleteAccountId}
        onCancel={() => setPendingDeleteAccountId(null)}
        onConfirm={(accountId) => void confirmDelete(accountId)}
      />
    </>
  );
}
