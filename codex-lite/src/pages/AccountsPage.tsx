import { useEffect, useMemo, useState } from 'react';
import { Crown, KeyRound, RefreshCw, Search, Settings, UserCheck, UserPlus, Users } from 'lucide-react';
import { AccountRow } from '../components/account/AccountRow';
import { ConfirmSwitchModal } from '../components/account/ConfirmSwitchModal';
import { ConfirmDeleteModal } from '../components/account/ConfirmDeleteModal';
import { ImportDrawer } from '../components/import/ImportDrawer';
import { Button } from '../components/ui/Button';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { useCodexAccountsStore } from '../stores/useCodexAccountsStore';
import { useImportFlowStore } from '../stores/useImportFlowStore';
import { isOAuthAuthMode, type CodexAccountView } from '../types/codex';
import './AccountsPage.css';

function matchesQuery(account: CodexAccountView, query: string): boolean {
  const haystack = [account.displayName, account.email, account.accountId, account.id]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  return haystack.includes(query);
}

interface AccountsPageProps {
  onOpenSessions: () => void;
}

export function AccountsPage({ onOpenSessions }: AccountsPageProps) {
  const {
    accounts,
    loading,
    error,
    refreshingAll,
    refreshingAccountIds,
    switchingAccountId,
    deletingAccountId,
    loadAccounts,
    refreshAccountQuota,
    refreshAllQuotas,
    switchToAccount,
    deleteAccount,
  } = useCodexAccountsStore();
  const openImportDrawer = useImportFlowStore((state) => state.openDrawer);
  const openOAuthLogin = useImportFlowStore((state) => state.openOAuthLogin);
  const [pendingSwitchAccountId, setPendingSwitchAccountId] = useState<string | null>(null);
  const [pendingDeleteAccountId, setPendingDeleteAccountId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    void loadAccounts();
  }, [loadAccounts]);

  const oauthAccountCount = useMemo(
    () => accounts.filter((account) => isOAuthAuthMode(account.authMode)).length,
    [accounts],
  );
  const apiAccountCount = useMemo(
    () => accounts.filter((account) => account.authMode === 'api_key').length,
    [accounts],
  );
  const currentAccountCount = useMemo(
    () => accounts.filter((account) => account.isCurrent).length,
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
      <div className="content accounts-content">
        <section className="accounts-dashboard">
          {error ? <ErrorBanner error={error} /> : null}
          <div className="accounts-panel-header">
            <div>
              <h2 className="accounts-title">账号管理</h2>
            </div>
          </div>

          <div className="accounts-stats" aria-label="Account statistics">
            <article className="stat-card">
              <span className="stat-icon stat-icon-primary">
                <Users size={24} />
              </span>
              <span className="stat-label">总账号</span>
              <strong>{accounts.length}</strong>
              <span className="stat-meta">个账号</span>
            </article>
            <article className="stat-card">
              <span className="stat-icon stat-icon-blue">
                <Crown size={24} />
              </span>
              <span className="stat-label">OAuth 账号</span>
              <strong>{oauthAccountCount}</strong>
              <span className="stat-meta">个账号</span>
            </article>
            <article className="stat-card">
              <span className="stat-icon stat-icon-green">
                <KeyRound size={24} />
              </span>
              <span className="stat-label">API Key 账号</span>
              <strong>{apiAccountCount}</strong>
              <span className="stat-meta">个账号</span>
            </article>
            <article className="stat-card">
              <span className="stat-icon stat-icon-purple">
                <UserCheck size={24} />
              </span>
              <span className="stat-label">当前账号</span>
              <strong>{currentAccountCount}</strong>
              <span className="stat-meta">已启用</span>
            </article>
          </div>

          <div className="accounts-toolbar" aria-label="Account controls">
            <div className="account-search">
              <Search size={16} aria-hidden="true" />
              <input
                aria-label="Search accounts"
                placeholder="搜索账号"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
              />
            </div>
            <div className="accounts-toolbar-actions">
              <Button
                variant="secondary"
                aria-label="Refresh all"
                icon={<RefreshCw size={16} />}
                loading={refreshingAll}
                disabled={loading || oauthAccountCount === 0}
                onClick={() => void refreshAllQuotas()}
              >
                刷新
              </Button>
              <Button
                variant="secondary"
                aria-label="Session management"
                icon={<Settings size={16} />}
                onClick={onOpenSessions}
              >
                会话管理
              </Button>
              <Button variant="primary" aria-label="Add Account" icon={<UserPlus size={16} />} onClick={openImportDrawer}>
                添加账号
              </Button>
            </div>
          </div>

          <div className="account-card-grid">
            {loading ? <p className="account-list-message">正在加载账号...</p> : null}
            {!loading && accounts.length === 0 ? (
              <div className="empty-state">
                <h2>No Codex accounts yet</h2>
                <p>添加当前本地 Codex 授权，或导入另一个账号。</p>
                <Button variant="primary" aria-label="Add Account" icon={<UserPlus size={16} />} onClick={openImportDrawer}>
                  添加账号
                </Button>
              </div>
            ) : null}
            {!loading && accounts.length > 0 && filteredAccounts.length === 0 ? (
              <p className="account-list-message">没有匹配 “{searchQuery.trim()}” 的账号。</p>
            ) : null}
            {filteredAccounts.map((account) => (
              <AccountRow
                account={account}
                key={account.id}
                refreshing={refreshingAll || refreshingAccountIds.includes(account.id)}
                switching={switchingAccountId === account.id}
                deleting={deletingAccountId === account.id}
                onRefreshQuota={(accountId) => void refreshAccountQuota(accountId)}
                onSwitch={setPendingSwitchAccountId}
                onDelete={setPendingDeleteAccountId}
                onReauthenticate={() => void openOAuthLogin()}
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
