import { Bot, FileText, MessageCircle, Settings, Sparkles, Users, Wand2 } from 'lucide-react';
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

  return (
    <div className="app-shell">
      <nav className="app-nav" aria-label="Primary">
        <div className="app-brand">
          <div className="app-logo">A.</div>
          <strong>AI Accounts</strong>
        </div>

        <div className="nav-section">
          <span className="nav-section-label">平台</span>
          <button
            className={`nav-button ${page === 'accounts' || page === 'sessions' ? 'active' : ''}`}
            aria-label="Codex"
            title="Accounts"
            onClick={() => setPage('accounts')}
          >
            <span className="nav-icon nav-icon-chatgpt">
              <Users size={19} />
            </span>
            <span>Codex</span>
          </button>
          <button className="nav-button nav-button-muted" aria-label="Claude" title="Claude" disabled>
            <span className="nav-icon nav-icon-claude">AI</span>
            <span>Claude</span>
          </button>
          <button className="nav-button nav-button-muted" aria-label="Gemini" title="Gemini" disabled>
            <span className="nav-icon nav-icon-gemini">
              <Sparkles size={19} />
            </span>
            <span>Gemini</span>
          </button>
          <button className="nav-button nav-button-muted" aria-label="Automation" title="Automation" disabled>
            <span className="nav-icon nav-icon-auto">
              <Bot size={19} />
            </span>
            <span>Automation</span>
          </button>
          <button className="nav-button nav-button-muted" aria-label="Tools" title="Tools" disabled>
            <span className="nav-icon nav-icon-tools">
              <Wand2 size={19} />
            </span>
            <span>Tools</span>
          </button>
        </div>

        <div className="nav-footer">
          <button
            className={`nav-button ${page === 'logs' ? 'active' : ''}`}
            aria-label="Logs"
            title="Logs"
            onClick={() => setPage('logs')}
          >
            <span className="nav-icon">
              <FileText size={19} />
            </span>
            <span>日志</span>
          </button>
          <button
            className={`nav-button ${page === 'settings' ? 'active' : ''}`}
            aria-label="Settings"
            title="Settings"
            onClick={() => setPage('settings')}
          >
            <span className="nav-icon">
              <Settings size={19} />
            </span>
            <span>设置</span>
          </button>
        </div>
      </nav>
      <main className="app-main">
        <header className="topbar" data-tauri-drag-region onPointerDown={handleTopbarPointerDown}>
          <div className="topbar-title-group">
            <MessageCircle size={18} aria-hidden="true" />
            <h1 className="page-title">
              {page === 'accounts' || page === 'sessions'
                ? 'AI Accounts'
                : page === 'settings'
                  ? 'Settings'
                  : 'Logs'}
            </h1>
          </div>
        </header>
        {children}
      </main>
    </div>
  );
}
