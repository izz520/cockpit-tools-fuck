import { Button } from '../components/ui/Button';
import { openDataDir, openLogDir } from '../services/systemService';
import { useSettingsStore } from '../stores/useSettingsStore';
import type { SystemSnapshot } from '../types/system';
import { useEffect } from 'react';

const pathRows: Array<{ label: string; key: keyof SystemSnapshot }> = [
  { label: 'App data', key: 'appDataDir' },
  { label: 'Logs', key: 'logsDir' },
  { label: 'Accounts file', key: 'accountsFilePath' },
  { label: 'Settings file', key: 'settingsFilePath' },
  { label: 'Codex home', key: 'defaultCodexHome' },
  { label: 'Codex auth', key: 'defaultCodexAuthFile' },
];

export function SettingsPage() {
  const { snapshot, error, loading, detecting, loadSnapshot, detectPaths } = useSettingsStore();

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  return (
    <div className="content">
      <section className="panel" style={{ maxWidth: 760, width: '100%', padding: 20 }}>
        <h2 className="section-title">Settings</h2>
        <p className="muted">Local-only paths and privacy controls.</p>
        {error ? <p className="muted">{error.message}</p> : null}
        {snapshot ? (
          <div className="settings-grid">
            {pathRows.map((row) => (
              <div className="settings-row" key={row.key}>
                <span>{row.label}</span>
                <code>{String(snapshot[row.key])}</code>
              </div>
            ))}
            <div className="settings-row">
              <span>Auth file status</span>
              <strong>{snapshot.codexAuthFileExists ? 'Found' : 'Missing'}</strong>
            </div>
          </div>
        ) : (
          <p className="muted">{loading ? 'Loading local paths...' : 'No local path snapshot loaded.'}</p>
        )}
        <div className="toolbar-actions">
          <Button variant="secondary" loading={detecting} onClick={() => void detectPaths()}>
            Detect Codex paths
          </Button>
          <Button variant="secondary" onClick={() => void openDataDir()}>
            Open data directory
          </Button>
          <Button variant="secondary" onClick={() => void openLogDir()}>
            Open logs
          </Button>
        </div>
      </section>
    </div>
  );
}
