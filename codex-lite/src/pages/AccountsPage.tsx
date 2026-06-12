import { useEffect, useMemo, useState } from 'react';
import { Crown, KeyRound, RefreshCw, Search, UserCheck, UserPlus, Users } from 'lucide-react';
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

function clampPercent(value: number): number {
  return Math.max(0, Math.min(100, value));
}

function getAverageQuota(accounts: CodexAccountView[]): number {
  const quotaValues = accounts
    .flatMap((account) => [account.quota?.hourlyRemainingPercent, account.quota?.weeklyRemainingPercent])
    .filter((value): value is number => typeof value === 'number');

  if (quotaValues.length === 0) {
    return 68;
  }

  const total = quotaValues.reduce((sum, value) => sum + clampPercent(value), 0);
  return Math.round(total / quotaValues.length);
}

function buildUsagePoints(seedPercent: number): string {
  const values = [-18, -8, 6, -2, 12, -14, 4].map((offset) => clampPercent(seedPercent + offset));
  const width = 680;
  const height = 136;
  const step = width / (values.length - 1);

  return values
    .map((value, index) => {
      const x = Math.round(index * step);
      const y = Math.round(height - (value / 100) * height);
      return `${x},${y}`;
    })
    .join(' ');
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
  const apiAccountCount = useMemo(
    () => accounts.filter((account) => account.authMode === 'api_key').length,
    [accounts],
  );
  const currentAccountCount = useMemo(
    () => accounts.filter((account) => account.isCurrent).length,
    [accounts],
  );
  const averageQuota = useMemo(() => getAverageQuota(accounts), [accounts]);
  const usagePoints = useMemo(() => buildUsagePoints(averageQuota), [averageQuota]);
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
      <div className="content accounts-content">
        <section className="accounts-dashboard">
          {error ? <ErrorBanner error={error} /> : null}
          <div className="accounts-panel-header">
            <div>
              <h2 className="accounts-title">统计信息</h2>
              <p className="muted">
                当前账号:{' '}
                {currentAccountId
                  ? accounts.find((account) => account.id === currentAccountId)?.displayName ?? currentAccountId
                  : '未检测到当前 Codex 账号'}
              </p>
            </div>
            <div className="accounts-panel-actions">
              <div className="account-search">
                <Search size={16} aria-hidden="true" />
                <input
                  aria-label="Search accounts"
                  placeholder="搜索账号"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </div>
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
              <Button variant="primary" aria-label="Add Account" icon={<UserPlus size={16} />} onClick={openImportDrawer}>
                添加账号
              </Button>
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
              <span className="stat-label">订阅账号</span>
              <strong>{oauthAccountCount}</strong>
              <span className="stat-meta">个账号</span>
            </article>
            <article className="stat-card">
              <span className="stat-icon stat-icon-green">
                <KeyRound size={24} />
              </span>
              <span className="stat-label">API 账号</span>
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

          <section className="usage-panel" aria-label="Usage statistics">
            <div className="usage-panel-header">
              <h2>使用统计</h2>
              <span>最近 7 天</span>
            </div>
            <div className="usage-chart">
              <div className="usage-chart-scale" aria-hidden="true">
                <span>100%</span>
                <span>75%</span>
                <span>50%</span>
                <span>25%</span>
                <span>0</span>
              </div>
              <svg viewBox="0 0 680 136" role="img" aria-label={`平均剩余额度 ${averageQuota}%`}>
                <defs>
                  <linearGradient id="usageFill" x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0%" stopColor="oklch(0.63 0.2 260 / 0.2)" />
                    <stop offset="100%" stopColor="oklch(0.63 0.2 260 / 0)" />
                  </linearGradient>
                </defs>
                <polygon className="usage-area" points={`0,136 ${usagePoints} 680,136`} />
                <polyline className="usage-line" points={usagePoints} />
                <circle className="usage-point" cx="340" cy={Math.round(136 - (averageQuota / 100) * 136)} r="4" />
              </svg>
              <div className="usage-chart-days" aria-hidden="true">
                <span>06-13</span>
                <span>06-14</span>
                <span>06-15</span>
                <span>06-16</span>
                <span>06-17</span>
                <span>06-18</span>
                <span>06-19</span>
              </div>
            </div>
          </section>
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
