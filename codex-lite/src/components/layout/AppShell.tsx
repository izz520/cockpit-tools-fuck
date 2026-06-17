import { FileText, MessageCircle, Settings, Users } from 'lucide-react';
import { type PointerEvent, type ReactNode } from 'react';
import { startWindowDragging } from '../../services/windowService';

type Page = 'accounts' | 'sessions' | 'settings' | 'logs';

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

  const getPageTitle = () => {
    switch (page) {
      case 'accounts':
      case 'sessions':
        return 'AI Accounts';
      case 'settings':
        return '设置';
      case 'logs':
        return '日志';
      default:
        return 'AI Accounts';
    }
  };

  return (
    <div className="app-shell">
      <nav className="app-nav" aria-label="Primary">
        <div className="app-brand">
          <div className="app-logo">A.</div>
        </div>

        <div className="nav-section">
          <button
            className={`nav-button nav-button-accounts ${page === 'accounts' || page === 'sessions' ? 'active' : ''}`}
            aria-label="Accounts"
            title="Accounts"
            onClick={() => setPage('accounts')}
          >
            <Users size={20} />
          </button>
        </div>

        <div className="nav-footer">
          <button
            className={`nav-button ${page === 'logs' ? 'active' : ''}`}
            aria-label="Logs"
            title="Logs"
            onClick={() => setPage('logs')}
          >
            <FileText size={20} />
          </button>
          <button
            className={`nav-button ${page === 'settings' ? 'active' : ''}`}
            aria-label="Settings"
            title="Settings"
            onClick={() => setPage('settings')}
          >
            <Settings size={20} />
          </button>
        </div>
      </nav>
      <main className="app-main">
        <header className="topbar" data-tauri-drag-region onPointerDown={handleTopbarPointerDown}>
          <div className="topbar-content">
            <div className="topbar-title-group">
              <MessageCircle size={18} aria-hidden="true" />
              <h1 className="page-title">{getPageTitle()}</h1>
            </div>
          </div>
        </header>
        {children}
      </main>
    </div>
  );
}
