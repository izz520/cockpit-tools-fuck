import { FileText, Settings, Users } from 'lucide-react';
import { type PointerEvent, type ReactNode } from 'react';
import { startWindowDragging } from '../../services/windowService';

type Page = 'accounts' | 'settings' | 'logs';

interface AppShellProps {
  page: Page;
  setPage: (page: Page) => void;
  children: ReactNode;
}

export function AppShell({ page, setPage, children }: AppShellProps) {
  const handleTopbarPointerDown = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }

    void startWindowDragging();
  };

  return (
    <div className="app-shell">
      <nav className="app-nav" aria-label="Primary">
        <div className="app-logo">C</div>
        <button className={`nav-button ${page === 'accounts' ? 'active' : ''}`} title="Accounts" onClick={() => setPage('accounts')}>
          <Users size={20} />
        </button>
        <button className={`nav-button ${page === 'settings' ? 'active' : ''}`} title="Settings" onClick={() => setPage('settings')}>
          <Settings size={20} />
        </button>
        <button className={`nav-button ${page === 'logs' ? 'active' : ''}`} title="Logs" onClick={() => setPage('logs')}>
          <FileText size={20} />
        </button>
      </nav>
      <main className="app-main">
        <header className="topbar" data-tauri-drag-region onPointerDown={handleTopbarPointerDown}>
          <div className="topbar-title-group">
            <h1 className="page-title">{page === 'accounts' ? 'Codex accounts' : page === 'settings' ? 'Settings' : 'Logs'}</h1>
          </div>
        </header>
        {children}
      </main>
    </div>
  );
}
