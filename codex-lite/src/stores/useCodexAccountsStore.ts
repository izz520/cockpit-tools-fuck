import { create } from 'zustand';
import { isOAuthAuthMode, type CodexAccountView } from '../types/codex';
import type { AppError } from '../types/system';
import {
  deleteCodexAccount,
  getCurrentCodexAccount,
  listCodexAccounts,
  refreshAllCodexQuotas,
  refreshCodexQuota,
  switchCodexAccount,
} from '../services/codexAccountService';

interface CodexAccountsState {
  accounts: CodexAccountView[];
  selectedAccountId: string | null;
  currentAccountId: string | null;
  loading: boolean;
  refreshingAll: boolean;
  refreshingAccountIds: string[];
  switchingAccountId: string | null;
  deletingAccountId: string | null;
  error: AppError | null;
  loadAccounts: () => Promise<void>;
  selectAccount: (accountId: string) => void;
  refreshAccountQuota: (accountId: string) => Promise<void>;
  refreshAllQuotas: () => Promise<void>;
  switchToAccount: (accountId: string) => Promise<void>;
  deleteAccount: (accountId: string) => Promise<void>;
  upsertAccounts: (accounts: CodexAccountView[]) => void;
}

function getCurrentId(accounts: CodexAccountView[]): string | null {
  return accounts.find((account) => account.isCurrent)?.id ?? null;
}

function mergeAccount(accounts: CodexAccountView[], updated: CodexAccountView): CodexAccountView[] {
  const existing = accounts.findIndex((account) => account.id === updated.id);
  if (existing === -1) {
    return [updated, ...accounts];
  }

  return accounts.map((account) => (account.id === updated.id ? updated : account));
}

export const useCodexAccountsStore = create<CodexAccountsState>((set, get) => ({
  accounts: [],
  selectedAccountId: null,
  currentAccountId: null,
  loading: false,
  refreshingAll: false,
  refreshingAccountIds: [],
  switchingAccountId: null,
  deletingAccountId: null,
  error: null,
  async loadAccounts() {
    set({ loading: true, error: null });
    try {
      const [accounts, current] = await Promise.all([
        listCodexAccounts(),
        getCurrentCodexAccount().catch(() => null),
      ]);
      const currentId = current?.id ?? getCurrentId(accounts);
      set({
        accounts,
        currentAccountId: currentId,
        selectedAccountId: get().selectedAccountId ?? currentId ?? accounts[0]?.id ?? null,
        loading: false,
      });
    } catch (error) {
      set({ error: error as AppError, loading: false });
    }
  },
  selectAccount(accountId) {
    set({ selectedAccountId: accountId });
  },
  async refreshAccountQuota(accountId) {
    set((state) => ({
      error: null,
      refreshingAccountIds: state.refreshingAccountIds.includes(accountId)
        ? state.refreshingAccountIds
        : [...state.refreshingAccountIds, accountId],
      accounts: state.accounts.map((account) =>
        account.id === accountId ? { ...account, quotaError: null } : account,
      ),
    }));
    try {
      const updatedAccount = await refreshCodexQuota(accountId);
      set((state) => ({
        refreshingAccountIds: state.refreshingAccountIds.filter((id) => id !== accountId),
        accounts: state.accounts.map((account) => (account.id === accountId ? updatedAccount : account)),
      }));
    } catch (error) {
      const appError = error as AppError;
      set((state) => ({
        error: appError,
        refreshingAccountIds: state.refreshingAccountIds.filter((id) => id !== accountId),
        accounts: state.accounts.map((account) =>
          account.id === accountId
            ? {
                ...account,
                quota: account.quota ? { ...account.quota, stale: true } : account.quota,
                quotaError: appError,
              }
            : account,
        ),
      }));
    }
  },
  async refreshAllQuotas() {
    const oauthAccountIds = get()
      .accounts.filter((account) => isOAuthAuthMode(account.authMode))
      .map((account) => account.id);
    set({ refreshingAll: true, error: null });
    try {
      const accounts = await refreshAllCodexQuotas();
      set({
        accounts,
        currentAccountId: getCurrentId(accounts),
        refreshingAll: false,
        refreshingAccountIds: [],
      });
    } catch (error) {
      const appError = error as AppError;
      set((state) => ({
        error: appError,
        refreshingAll: false,
        refreshingAccountIds: state.refreshingAccountIds.filter((id) => !oauthAccountIds.includes(id)),
      }));
    }
  },
  async switchToAccount(accountId) {
    set({ switchingAccountId: accountId, error: null });
    try {
      const result = await switchCodexAccount(accountId);
      set((state) => {
        const accounts = state.accounts.map((account) => ({
          ...account,
          isCurrent: account.id === result.account.id,
        }));
        return {
          accounts: mergeAccount(accounts, result.account),
          currentAccountId: result.account.id,
          selectedAccountId: result.account.id,
          switchingAccountId: null,
        };
      });
    } catch (error) {
      set({ error: error as AppError, switchingAccountId: null });
    }
  },
  async deleteAccount(accountId) {
    set({ deletingAccountId: accountId, error: null });
    try {
      await deleteCodexAccount(accountId);
      set((state) => {
        const accounts = state.accounts.filter((account) => account.id !== accountId);
        const nextSelected =
          state.selectedAccountId === accountId
            ? getCurrentId(accounts) ?? accounts[0]?.id ?? null
            : state.selectedAccountId;
        return {
          accounts,
          currentAccountId:
            state.currentAccountId === accountId ? getCurrentId(accounts) : state.currentAccountId,
          selectedAccountId: nextSelected,
          deletingAccountId: null,
        };
      });
    } catch (error) {
      set({ error: error as AppError, deletingAccountId: null });
    }
  },
  upsertAccounts(nextAccounts) {
    set((state) => {
      const merged = nextAccounts.reduce(mergeAccount, state.accounts);
      return {
        accounts: merged,
        selectedAccountId: nextAccounts[0]?.id ?? state.selectedAccountId,
        currentAccountId: getCurrentId(merged),
      };
    });
  },
}));
