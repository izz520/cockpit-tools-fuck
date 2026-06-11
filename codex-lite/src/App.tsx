import { useState } from 'react';
import { AppShell } from './components/layout/AppShell';
import { AccountsPage } from './pages/AccountsPage';
import { LogsPage } from './pages/LogsPage';
import { SettingsPage } from './pages/SettingsPage';

type Page = 'accounts' | 'settings' | 'logs';

export function App() {
  const [page, setPage] = useState<Page>('accounts');

  return (
    <AppShell page={page} setPage={setPage}>
      {page === 'accounts' ? <AccountsPage /> : null}
      {page === 'settings' ? <SettingsPage /> : null}
      {page === 'logs' ? <LogsPage /> : null}
    </AppShell>
  );
}
